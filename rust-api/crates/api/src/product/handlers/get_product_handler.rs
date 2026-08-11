use axum::extract::{Path, State};
use kernel::AppError;
use uuid::Uuid;

use domain::product::ProductId;

use crate::error_response::HttpError;
use crate::json::Sonic;
use crate::product::dto::ProductResponse;
use crate::response::ApiResponse;
use crate::state::AppState;

#[tracing::instrument(name = "handler.get_product", skip(state))]
pub async fn get_product_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Sonic<ApiResponse<ProductResponse>>, HttpError> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| AppError::validation("product/invalid-id", "id must be a valid uuid"))?;

    let product = state
        .get_product
        .execute(ProductId::from_uuid(uuid))
        .await?;

    Ok(Sonic(ApiResponse::ok(ProductResponse {
        id: product.id.to_string(),
        owner_id: product.owner_id.to_string(),
        name: product.name,
        description: product.description,
        price_cents: product.price_cents,
        currency: product.currency,
        created_at: product.created_at.to_rfc3339(),
    })))
}
