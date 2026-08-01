use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use kernel::AppError;
use serde::Serialize;
use sonic_rs::Value;

use crate::json::Sonic;

#[derive(Serialize)]
struct ApiErrorBody {
    ok: bool,
    error: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<Value>,
}

/// Newtype so `api` (not `kernel`) owns the HTTP mapping — keeps the shared
/// error type transport-agnostic. `details` is only ever populated for
/// client-caused errors (validation); `internal/server-error` never leaks
/// its cause — that goes to `tracing` only.
pub struct HttpError(pub AppError);

impl From<AppError> for HttpError {
    fn from(err: AppError) -> Self {
        Self(err)
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let code = self.0.code();

        let (status, details) = match &self.0 {
            AppError::Validation { issues, .. } => (
                StatusCode::UNPROCESSABLE_ENTITY,
                issues_to_value(issues),
            ),
            AppError::NotFound { .. } => (StatusCode::NOT_FOUND, None),
            AppError::Conflict { .. } => (StatusCode::CONFLICT, None),
            AppError::Unauthorized { .. } => (StatusCode::UNAUTHORIZED, None),
            AppError::Forbidden { .. } => (StatusCode::FORBIDDEN, None),
            AppError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, None),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, None),
        };

        if let AppError::Internal(err) = &self.0 {
            tracing::error!(error = %err, code, "internal error");
        } else {
            tracing::warn!(code, message = %self.0, "request rejected");
        }

        (
            status,
            Sonic(ApiErrorBody {
                ok: false,
                error: code,
                details,
            }),
        )
            .into_response()
    }
}

fn issues_to_value(issues: &[kernel::ValidationIssue]) -> Option<Value> {
    if issues.is_empty() {
        return None;
    }
    sonic_rs::to_value(issues).ok()
}
