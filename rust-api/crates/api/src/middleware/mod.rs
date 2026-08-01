mod panic_handler;
mod rate_limit;
mod request_id;
mod request_logging;
mod security_headers;

use axum::http::{HeaderValue, Method};
use axum::Router;
use infrastructure::config::AppConfig;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use tower::limit::ConcurrencyLimitLayer;
use std::time::Duration;

const MAX_IN_FLIGHT_REQUESTS: usize = 4096;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// Cross-cutting HTTP concerns, applied outside-in (request order, first to
/// last): request id → panic safety net → CORS → security headers → body
/// size cap → rate limit → request logging span → compression →
/// concurrency cap → timeout → request-id response propagation → router.
pub fn apply(router: Router, config: &AppConfig) -> Router {
    let rate_limiter = rate_limit::RateLimitLayer::new(
        config.rate_limit_per_minute,
        config.rate_limit_burst,
    );

    router
        .layer(request_id::propagate_request_id_layer())
        .layer(TimeoutLayer::new(REQUEST_TIMEOUT))
        .layer(ConcurrencyLimitLayer::new(MAX_IN_FLIGHT_REQUESTS))
        .layer(CompressionLayer::new())
        .layer(axum::middleware::from_fn(request_logging::request_logging))
        .layer(axum::middleware::from_fn(move |req, next| {
            let limiter = rate_limiter.clone();
            async move { limiter.enforce(req, next).await }
        }))
        .layer(RequestBodyLimitLayer::new(config.max_body_bytes))
        .layer(axum::middleware::from_fn(
            security_headers::security_headers,
        ))
        .layer(build_cors_layer(config))
        .layer(CatchPanicLayer::custom(panic_handler::handle_panic))
        .layer(request_id::set_request_id_layer())
}

fn build_cors_layer(config: &AppConfig) -> CorsLayer {
    let origins = config.cors_origins();

    let allow_origin = if origins.is_empty() {
        AllowOrigin::list(Vec::<HeaderValue>::new())
    } else {
        let parsed: Vec<HeaderValue> = origins
            .iter()
            .filter_map(|origin| HeaderValue::from_str(origin).ok())
            .collect();
        AllowOrigin::list(parsed)
    };

    CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderName::from_static("x-request-id"),
        ])
        .expose_headers([axum::http::HeaderName::from_static("x-request-id")])
        .max_age(Duration::from_secs(3600))
}
