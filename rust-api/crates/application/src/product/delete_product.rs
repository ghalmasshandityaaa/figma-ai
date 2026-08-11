use std::sync::Arc;

use domain::product::{ProductId, ProductRepository};
use domain::user::UserId;
use kernel::{AppError, AppResult};

/// One use case, one action: permanently remove a listing. Only the
/// product's owner may delete it — `requester_id` comes from the caller's
/// already-verified `UserId` (see the `AuthenticatedUser` extractor), never
/// a client-supplied field.
pub struct DeleteProduct {
    repository: Arc<dyn ProductRepository>,
}

impl DeleteProduct {
    pub fn new(repository: Arc<dyn ProductRepository>) -> Self {
        Self { repository }
    }

    #[tracing::instrument(name = "usecase.delete_product", skip_all, fields(product_id = %id, requester_id = %requester_id))]
    pub async fn execute(&self, id: ProductId, requester_id: UserId) -> AppResult<()> {
        let product = self
            .repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::not_found("product/not-found", "product not found"))?;

        if product.owner_id != requester_id {
            return Err(AppError::forbidden(
                "product/not-owner",
                "only the owner may delete this product",
            ));
        }

        let deleted = self.repository.delete(id).await?;
        if !deleted {
            return Err(AppError::not_found("product/not-found", "product not found"));
        }

        tracing::info!(product_id = %id, "product deleted");
        Ok(())
    }
}
