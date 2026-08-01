use std::sync::Arc;

use sqlx::{PgPool, Postgres, Transaction};
use tokio::sync::Mutex;

/// Abstracts "run against the pool" vs "run against an already-open
/// transaction", so one repository implementation works both standalone
/// and as a `TransactionScope` participant inside `UnitOfWork::execute`.
#[derive(Clone)]
pub enum DbHandle {
    Pool(PgPool),
    Tx(Arc<Mutex<Transaction<'static, Postgres>>>),
}
