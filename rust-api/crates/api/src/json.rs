use axum::body::Bytes;
use axum::extract::{FromRequest, Request};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::de::DeserializeOwned;
use serde::Serialize;

/// JSON extractor/responder backed by `sonic-rs` (SIMD-accelerated) instead
/// of `serde_json` — drop-in replacement, meaningfully faster on both
/// decode and encode paths under load.
pub struct Sonic<T>(pub T);

#[axum::async_trait]
impl<T, S> FromRequest<S> for Sonic<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(IntoResponse::into_response)?;

        let value = sonic_rs::from_slice(&bytes).map_err(|err| {
            (StatusCode::BAD_REQUEST, format!("invalid json body: {err}")).into_response()
        })?;

        Ok(Sonic(value))
    }
}

impl<T: Serialize> IntoResponse for Sonic<T> {
    fn into_response(self) -> Response {
        match sonic_rs::to_vec(&self.0) {
            Ok(bytes) => {
                ([(header::CONTENT_TYPE, "application/json")], bytes).into_response()
            }
            Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
        }
    }
}
