use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use domain::user::UserRepository;
use kernel::AppResult;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Repository handles bound to one open, not-yet-committed transaction.
/// Every call made through these handles participates in the same
/// commit/rollback — do not let them escape the `UnitOfWork::execute`
/// closure.
pub trait TransactionScope: Send + Sync {
    fn users(&self) -> Arc<dyn UserRepository>;
}

/// Runs `work` inside a single atomic database transaction: commits if
/// `work` returns `Ok`, rolls back (including on error propagated via `?`)
/// otherwise. This is the Unit-of-Work pattern — use it whenever a use
/// case must make more than one repository call as one atomic operation.
#[async_trait]
pub trait UnitOfWork: Send + Sync + 'static {
    async fn execute(
        &self,
        work: Box<dyn FnOnce(Arc<dyn TransactionScope>) -> BoxFuture<'static, AppResult<()>> + Send>,
    ) -> AppResult<()>;
}
