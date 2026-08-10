use axum::extract::{Path, State};
use kernel::AppError;
use uuid::Uuid;

use domain::user::UserId;

use crate::user::dto::ProfileResponse;
use crate::error_response::HttpError;
use crate::json::Sonic;
use crate::response::ApiResponse;
use crate::state::AppState;

#[tracing::instrument(name = "handler.get_profile", skip(state))]
pub async fn get_profile_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Sonic<ApiResponse<ProfileResponse>>, HttpError> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| AppError::validation("user/invalid-id", "id must be a valid uuid"))?;

    let profile = state
        .get_user_profile
        .execute(UserId::from_uuid(uuid))
        .await?;

    Ok(Sonic(ApiResponse::ok(ProfileResponse {
        id: profile.id.to_string(),
        username: profile.username,
        email: profile.email,
        is_active: profile.is_active,
        created_at: profile.created_at.to_rfc3339(),
    })))
}
