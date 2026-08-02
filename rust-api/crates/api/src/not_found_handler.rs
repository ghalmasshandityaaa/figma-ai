use kernel::AppError;

use crate::error_response::HttpError;

/// Router-wide fallback for unmatched routes.
pub async fn not_found_handler() -> HttpError {
    HttpError(AppError::not_found(
        "route/not-found",
        "the requested route does not exist",
    ))
}
