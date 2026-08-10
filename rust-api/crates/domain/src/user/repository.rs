use async_trait::async_trait;
use kernel::AppResult;

use super::{
    entity::User,
    events::UserEvent,
    value_objects::{Email, UserId, Username},
};

/// Port owned by the domain, implemented by infrastructure. Persists the
/// aggregate and its outbox event atomically (transactional outbox).
#[async_trait]
pub trait UserRepository: Send + Sync + 'static {
    async fn insert_with_event(&self, user: &User, event: &UserEvent) -> AppResult<()>;
    /// Deletes the user in the same transaction as its outbox event.
    /// Returns `false` (no event written) if no row matched `id`.
    async fn delete_with_event(&self, id: UserId, event: &UserEvent) -> AppResult<bool>;
    async fn find_by_id(&self, id: UserId) -> AppResult<Option<User>>;
    async fn find_by_email(&self, email: &Email) -> AppResult<Option<User>>;
    async fn find_by_username(&self, username: &Username) -> AppResult<Option<User>>;
    /// Returns a page of users ordered by creation time, plus the total
    /// row count needed to compute `PageMetadata`.
    async fn list_page(&self, limit: i64, offset: i64) -> AppResult<(Vec<User>, i64)>;
}
