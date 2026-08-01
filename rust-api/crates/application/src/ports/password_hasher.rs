use async_trait::async_trait;
use domain::user::{PasswordHash, RawPassword};
use kernel::AppResult;

/// Hashing is CPU-bound (Argon2id); the adapter is expected to run it via
/// `spawn_blocking` so it never stalls the async executor.
#[async_trait]
pub trait PasswordHasher: Send + Sync + 'static {
    async fn hash(&self, raw: &RawPassword) -> AppResult<PasswordHash>;
    async fn verify(&self, raw: &RawPassword, hash: &PasswordHash) -> AppResult<bool>;
}
