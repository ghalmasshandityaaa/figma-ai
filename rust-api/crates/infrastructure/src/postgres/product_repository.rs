use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::product::{Money, Product, ProductId, ProductName, ProductRepository};
use domain::user::UserId;
use kernel::{AppError, AppResult};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use super::db_handle::DbHandle;

pub struct PgProductRepository {
    db: DbHandle,
}

impl PgProductRepository {
    pub fn new(pool: PgPool) -> Self {
        Self {
            db: DbHandle::Pool(pool),
        }
    }
}

#[derive(FromRow)]
struct ProductRow {
    id: Uuid,
    owner_id: Uuid,
    name: String,
    description: Option<String>,
    price_cents: i64,
    currency: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<ProductRow> for Product {
    fn from(row: ProductRow) -> Self {
        Product {
            id: ProductId::from_uuid(row.id),
            owner_id: UserId::from_uuid(row.owner_id),
            name: ProductName::parse(row.name).expect("persisted product name violates invariant"),
            description: row.description,
            money: Money::new(row.price_cents, row.currency)
                .expect("persisted money violates invariant"),
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[async_trait]
impl ProductRepository for PgProductRepository {
    #[tracing::instrument(name = "repository.insert", skip_all, fields(product_id = %product.id))]
    async fn insert(&self, product: &Product) -> AppResult<()> {
        let query = sqlx::query(
            r#"
            insert into products (id, owner_id, name, description, price_cents, currency, created_at, updated_at)
            values ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(product.id.as_uuid())
        .bind(product.owner_id.as_uuid())
        .bind(product.name.as_str())
        .bind(product.description.as_deref())
        .bind(product.money.price_cents())
        .bind(product.money.currency())
        .bind(product.created_at)
        .bind(product.updated_at);

        match &self.db {
            DbHandle::Pool(pool) => query.execute(pool).await,
            DbHandle::Tx(tx) => query.execute(&mut **tx.lock().await).await,
        }
        .map_err(map_sqlx_err)?;

        Ok(())
    }

    #[tracing::instrument(name = "repository.find_by_id", skip_all, fields(product_id = %id))]
    async fn find_by_id(&self, id: ProductId) -> AppResult<Option<Product>> {
        let query = sqlx::query_as::<_, ProductRow>("select * from products where id = $1")
            .bind(id.as_uuid());

        let row = match &self.db {
            DbHandle::Pool(pool) => query.fetch_optional(pool).await,
            DbHandle::Tx(tx) => query.fetch_optional(&mut **tx.lock().await).await,
        }
        .map_err(|e| AppError::Internal(e.into()))?;

        Ok(row.map(Product::from))
    }

    #[tracing::instrument(name = "repository.list_page", skip_all, fields(limit, offset))]
    async fn list_page(&self, limit: i64, offset: i64) -> AppResult<(Vec<Product>, i64)> {
        // Ordered by id (UUIDv7, time-sortable) descending — newest-first
        // without a second index, since the primary key already covers it.
        let list_query = sqlx::query_as::<_, ProductRow>(
            "select * from products order by id desc limit $1 offset $2",
        )
        .bind(limit)
        .bind(offset);

        let rows = match &self.db {
            DbHandle::Pool(pool) => list_query.fetch_all(pool).await,
            DbHandle::Tx(tx) => list_query.fetch_all(&mut **tx.lock().await).await,
        }
        .map_err(|e| AppError::Internal(e.into()))?;

        let count_query = sqlx::query_scalar::<_, i64>("select count(*) from products");
        let total = match &self.db {
            DbHandle::Pool(pool) => count_query.fetch_one(pool).await,
            DbHandle::Tx(tx) => count_query.fetch_one(&mut **tx.lock().await).await,
        }
        .map_err(|e| AppError::Internal(e.into()))?;

        Ok((rows.into_iter().map(Product::from).collect(), total))
    }
}

fn map_sqlx_err(err: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(db_err) = &err {
        if db_err.is_foreign_key_violation() {
            return AppError::not_found("product/owner-not-found", "owner not found");
        }
    }
    AppError::Internal(err.into())
}
