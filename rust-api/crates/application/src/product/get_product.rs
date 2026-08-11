use std::sync::Arc;

use chrono::{DateTime, Utc};
use domain::product::{ProductId, ProductRepository};
use domain::user::UserId;
use kernel::{AppError, AppResult};

pub struct ProductProfile {
    pub id: ProductId,
    pub owner_id: UserId,
    pub name: String,
    pub description: Option<String>,
    pub price_cents: i64,
    pub currency: String,
    pub created_at: DateTime<Utc>,
}

/// One use case, one action: read a single product.
pub struct GetProduct {
    repository: Arc<dyn ProductRepository>,
}

impl GetProduct {
    pub fn new(repository: Arc<dyn ProductRepository>) -> Self {
        Self { repository }
    }

    #[tracing::instrument(name = "usecase.get_product", skip_all, fields(product_id = %id))]
    pub async fn execute(&self, id: ProductId) -> AppResult<ProductProfile> {
        let product = self
            .repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::not_found("product/not-found", "product not found"))?;

        Ok(ProductProfile {
            id: product.id,
            owner_id: product.owner_id,
            name: product.name.as_str().to_owned(),
            description: product.description,
            price_cents: product.money.price_cents(),
            currency: product.money.currency().to_owned(),
            created_at: product.created_at,
        })
    }
}
