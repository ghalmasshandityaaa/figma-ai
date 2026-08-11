use axum::extract::{Query, State};
use kernel::pagination::{PageMetadata, PageRequest};

use crate::error_response::HttpError;
use crate::json::Sonic;
use crate::product::dto::ProductResponse;
use crate::response::ApiResponse;
use crate::state::AppState;

#[tracing::instrument(name = "handler.list_products", skip_all, fields(page = page.page, page_size = page.page_size))]
pub async fn list_products_handler(
    State(state): State<AppState>,
    Query(page): Query<PageRequest>,
) -> Result<Sonic<ApiResponse<Vec<ProductResponse>>>, HttpError> {
    let output = state
        .list_products
        .execute(page.limit(), page.offset())
        .await?;

    let items = output
        .items
        .into_iter()
        .map(|product| ProductResponse {
            id: product.id.to_string(),
            owner_id: product.owner_id.to_string(),
            name: product.name,
            description: product.description,
            price_cents: product.price_cents,
            currency: product.currency,
            created_at: product.created_at.to_rfc3339(),
        })
        .collect();

    let metadata = PageMetadata::new(page, output.total);
    Ok(Sonic(ApiResponse::paginated(items, metadata)))
}
