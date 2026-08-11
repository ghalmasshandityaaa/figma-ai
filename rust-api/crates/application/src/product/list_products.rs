use std::sync::Arc;

use domain::product::ProductRepository;
use kernel::AppResult;

use super::ProductProfile;

pub struct ListProductsOutput {
    pub items: Vec<ProductProfile>,
    pub total: i64,
}

/// One use case, one action: list products, paginated.
pub struct ListProducts {
    repository: Arc<dyn ProductRepository>,
}

impl ListProducts {
    pub fn new(repository: Arc<dyn ProductRepository>) -> Self {
        Self { repository }
    }

    #[tracing::instrument(name = "usecase.list_products", skip_all, fields(limit, offset))]
    pub async fn execute(&self, limit: i64, offset: i64) -> AppResult<ListProductsOutput> {
        let (products, total) = self.repository.list_page(limit, offset).await?;

        let items = products
            .into_iter()
            .map(|product| ProductProfile {
                id: product.id,
                owner_id: product.owner_id,
                name: product.name.as_str().to_owned(),
                description: product.description,
                price_cents: product.money.price_cents(),
                currency: product.money.currency().to_owned(),
                created_at: product.created_at,
            })
            .collect();

        Ok(ListProductsOutput { items, total })
    }
}
