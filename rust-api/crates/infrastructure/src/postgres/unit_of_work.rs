use std::sync::Arc;

use application::ports::{BoxFuture, TransactionScope, UnitOfWork};
use async_trait::async_trait;
use domain::user::UserRepository;
use kernel::{AppError, AppResult};
use sqlx::PgPool;
use tokio::sync::Mutex;

use super::db_handle::DbHandle;
use super::user_repository::PgUserRepository;

pub struct PgUnitOfWork {
    pool: PgPool,
}

impl PgUnitOfWork {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UnitOfWork for PgUnitOfWork {
    #[tracing::instrument(skip_all, name = "pg_unit_of_work")]
    async fn execute(
        &self,
        work: Box<dyn FnOnce(Arc<dyn TransactionScope>) -> BoxFuture<'static, AppResult<()>> + Send>,
    ) -> AppResult<()> {
        let tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::Internal(e.into()))?;
        let shared_tx = Arc::new(Mutex::new(tx));

        let scope: Arc<dyn TransactionScope> = Arc::new(PgTransactionScope {
            tx: shared_tx.clone(),
        });

        let result = work(scope).await;

        let tx = Arc::try_unwrap(shared_tx)
            .map_err(|_| {
                AppError::Internal(anyhow::anyhow!(
                    "transaction scope outlived UnitOfWork::execute — a repository handle escaped the closure"
                ))
            })?
            .into_inner();

        match result {
            Ok(()) => tx.commit().await.map_err(|e| AppError::Internal(e.into())),
            Err(err) => {
                let _ = tx.rollback().await;
                Err(err)
            }
        }
    }
}

struct PgTransactionScope {
    tx: Arc<Mutex<sqlx::Transaction<'static, sqlx::Postgres>>>,
}

impl TransactionScope for PgTransactionScope {
    fn users(&self) -> Arc<dyn UserRepository> {
        Arc::new(PgUserRepository::from_handle(DbHandle::Tx(self.tx.clone())))
    }
}
