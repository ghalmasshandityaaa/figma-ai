use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use kernel::AppError;
use uuid::Uuid;

use domain::user::UserId;

use crate::user::dto::UserStatusResponse;
use crate::error_response::HttpError;
use crate::json::Sonic;
use crate::response::ApiResponse;
use crate::state::AppState;

#[tracing::instrument(name = "handler.deactivate_user", skip(state))]
pub async fn deactivate_user_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, HttpError> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| AppError::validation("user/invalid-id", "id must be a valid uuid"))?;
    let user_id = UserId::from_uuid(uuid);

    state.deactivate_user.execute(user_id).await?;

    let response = UserStatusResponse {
        id: user_id.to_string(),
        is_active: false,
    };
    Ok((StatusCode::OK, Sonic(ApiResponse::ok(response))).into_response())
}
