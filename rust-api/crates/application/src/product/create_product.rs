use std::sync::Arc;

use chrono::{DateTime, Utc};
use domain::product::{Money, Product, ProductId, ProductName, ProductRepository};
use domain::user::{UserId, UserRepository};
use kernel::{AppError, AppResult};

pub struct CreateProductCommand {
    pub owner_id: UserId,
    pub name: String,
    pub description: Option<String>,
    pub price_cents: i64,
    pub currency: String,
}

pub struct CreateProductOutput {
    pub id: ProductId,
    pub owner_id: UserId,
    pub name: String,
    pub description: Option<String>,
    pub price_cents: i64,
    pub currency: String,
    pub created_at: DateTime<Utc>,
}

/// One use case, one action: list a new product for sale. `owner_id` comes
/// from the caller's already-verified `UserId` (see the `AuthenticatedUser`
/// extractor) — it is never trusted from the request body.
pub struct CreateProduct {
    repository: Arc<dyn ProductRepository>,
    users: Arc<dyn UserRepository>,
}

impl CreateProduct {
    pub fn new(repository: Arc<dyn ProductRepository>, users: Arc<dyn UserRepository>) -> Self {
        Self { repository, users }
    }

    #[tracing::instrument(name = "usecase.create_product", skip_all, fields(owner_id = %cmd.owner_id))]
    pub async fn execute(&self, cmd: CreateProductCommand) -> AppResult<CreateProductOutput> {
        let name = ProductName::parse(cmd.name)?;
        let money = Money::new(cmd.price_cents, cmd.currency)?;

        // A verified token only proves the subject claim once decrypted
        // correctly (ADR-0001) — it never checks the users table. Reject a
        // token for an owner that no longer exists with a clean AppError
        // instead of letting the insert fail on the `owner_id` FK.
        if self.users.find_by_id(cmd.owner_id).await?.is_none() {
            return Err(AppError::not_found(
                "product/owner-not-found",
                "owner not found",
            ));
        }

        let now = Utc::now();
        let product = Product::create(cmd.owner_id, name, cmd.description, money, now)?;

        self.repository.insert(&product).await?;

        tracing::info!(product_id = %product.id, owner_id = %product.owner_id, "product created");

        Ok(CreateProductOutput {
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
