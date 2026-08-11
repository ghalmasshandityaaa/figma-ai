use axum::extract::{Path, State};
use axum::http::StatusCode;
use kernel::AppError;
use uuid::Uuid;

use domain::product::ProductId;

use crate::authenticated_user::AuthenticatedUser;
use crate::error_response::HttpError;
use crate::state::AppState;

#[tracing::instrument(name = "handler.delete_product", skip_all, fields(owner_id = %owner_id))]
pub async fn delete_product_handler(
    State(state): State<AppState>,
    AuthenticatedUser(owner_id): AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<StatusCode, HttpError> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| AppError::validation("product/invalid-id", "id must be a valid uuid"))?;

    state
        .delete_product
        .execute(ProductId::from_uuid(uuid), owner_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
