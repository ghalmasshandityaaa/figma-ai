use std::any::Any;

use axum::response::{IntoResponse, Response};
use kernel::AppError;

use crate::error_response::HttpError;

/// Converts an unwinding panic into the standard `internal/server-error`
/// envelope instead of crashing the process or dropping the connection.
/// The real panic message is logged; it never reaches the client.
pub fn handle_panic(payload: Box<dyn Any + Send + 'static>) -> Response {
    let reason = payload
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "non-string panic payload".to_string());

    tracing::error!(panic = %reason, "panic caught by CatchPanicLayer, request aborted safely");

    HttpError(AppError::Internal(anyhow::anyhow!("panic: {reason}"))).into_response()
}
