use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::num::NonZeroU32;
use std::sync::Arc;

use axum::extract::{ConnectInfo, Request};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use governor::clock::DefaultClock;
use governor::state::keyed::DefaultKeyedStateStore;
use governor::{Quota, RateLimiter};
use kernel::AppError;

use crate::error_response::HttpError;

type IpRateLimiter = RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>;

/// Per-client-IP token bucket. `check_key` is lock-free on the happy path
/// (dashmap-backed state store) — negligible overhead per request.
#[derive(Clone)]
pub struct RateLimitLayer {
    limiter: Arc<IpRateLimiter>,
}

impl RateLimitLayer {
    pub fn new(per_minute: u32, burst: u32) -> Self {
        let quota = Quota::per_minute(NonZeroU32::new(per_minute.max(1)).unwrap())
            .allow_burst(NonZeroU32::new(burst.max(1)).unwrap());

        Self {
            limiter: Arc::new(RateLimiter::keyed(quota)),
        }
    }

    pub async fn enforce(self, req: Request, next: Next) -> Response {
        let ip = client_ip(&req);

        match self.limiter.check_key(&ip) {
            Ok(_) => next.run(req).await,
            Err(_) => {
                tracing::warn!(%ip, "rate limit exceeded");
                HttpError(AppError::RateLimited).into_response()
            }
        }
    }
}

/// Trusts `X-Forwarded-For` (set by the reverse proxy in front of this
/// service); falls back to the direct peer address for local/dev use.
fn client_ip(req: &Request) -> IpAddr {
    req.headers()
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .and_then(|value| value.trim().parse().ok())
        .or_else(|| {
            req.extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|connect_info| connect_info.0.ip())
        })
        .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
}
