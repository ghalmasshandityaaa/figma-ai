use async_trait::async_trait;
use kernel::AppResult;

use super::{entity::Product, value_objects::ProductId};

/// Port owned by the domain, implemented by infrastructure. No
/// `insert_with_event` shape here — product creation is a plain
/// transactional insert (ADR-0001 / PRD non-goal: no outbox event yet).
#[async_trait]
pub trait ProductRepository: Send + Sync + 'static {
    async fn insert(&self, product: &Product) -> AppResult<()>;
    async fn find_by_id(&self, id: ProductId) -> AppResult<Option<Product>>;
    /// Returns a page of products ordered newest-first (id is UUIDv7, so
    /// this stays index-backed via the primary key), plus the total row
    /// count needed to compute `PageMetadata`.
    async fn list_page(&self, limit: i64, offset: i64) -> AppResult<(Vec<Product>, i64)>;
    /// Returns `true` if a row was deleted, `false` if no product with
    /// this id existed — lets the use case distinguish a genuine
    /// not-found from a successful delete without a prior read.
    async fn delete(&self, id: ProductId) -> AppResult<bool>;
}
