use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::{Email, PasswordHash, User, UserEvent, UserId, UserRepository, Username};
use kernel::{AppError, AppResult};
use sqlx::{FromRow, PgConnection, PgPool};
use uuid::Uuid;

use super::db_handle::DbHandle;

pub struct PgUserRepository {
    db: DbHandle,
}

impl PgUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self {
            db: DbHandle::Pool(pool),
        }
    }

    pub(crate) fn from_handle(db: DbHandle) -> Self {
        Self { db }
    }
}

#[derive(FromRow)]
struct UserRow {
    id: Uuid,
    username: String,
    email: String,
    password_hash: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<UserRow> for User {
    fn from(row: UserRow) -> Self {
        User {
            id: UserId::from_uuid(row.id),
            username: Username::parse(row.username)
                .expect("persisted username violates invariant"),
            email: Email::parse(row.email).expect("persisted email violates invariant"),
            password_hash: PasswordHash::from_hashed(row.password_hash),
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[async_trait]
impl UserRepository for PgUserRepository {
    async fn insert_with_event(&self, user: &User, event: &UserEvent) -> AppResult<()> {
        match &self.db {
            DbHandle::Pool(pool) => {
                let mut tx = pool.begin().await.map_err(|e| AppError::Internal(e.into()))?;
                run_insert_with_event(&mut tx, user, event)
                    .await
                    .map_err(map_sqlx_err)?;
                tx.commit().await.map_err(|e| AppError::Internal(e.into()))
            }
            DbHandle::Tx(tx) => {
                let mut guard = tx.lock().await;
                run_insert_with_event(&mut guard, user, event)
                    .await
                    .map_err(map_sqlx_err)
            }
        }
    }

    async fn find_by_id(&self, id: UserId) -> AppResult<Option<User>> {
        let query = sqlx::query_as::<_, UserRow>("select * from users where id = $1").bind(id.as_uuid());

        let row = match &self.db {
            DbHandle::Pool(pool) => query.fetch_optional(pool).await,
            DbHandle::Tx(tx) => query.fetch_optional(&mut **tx.lock().await).await,
        }
        .map_err(|e| AppError::Internal(e.into()))?;

        Ok(row.map(User::from))
    }

    async fn find_by_email(&self, email: &Email) -> AppResult<Option<User>> {
        let query =
            sqlx::query_as::<_, UserRow>("select * from users where email = $1").bind(email.as_str());

        let row = match &self.db {
            DbHandle::Pool(pool) => query.fetch_optional(pool).await,
            DbHandle::Tx(tx) => query.fetch_optional(&mut **tx.lock().await).await,
        }
        .map_err(|e| AppError::Internal(e.into()))?;

        Ok(row.map(User::from))
    }

    async fn find_by_username(&self, username: &Username) -> AppResult<Option<User>> {
        let query = sqlx::query_as::<_, UserRow>("select * from users where username = $1")
            .bind(username.as_str());

        let row = match &self.db {
            DbHandle::Pool(pool) => query.fetch_optional(pool).await,
            DbHandle::Tx(tx) => query.fetch_optional(&mut **tx.lock().await).await,
        }
        .map_err(|e| AppError::Internal(e.into()))?;

        Ok(row.map(User::from))
    }

    async fn list_page(&self, limit: i64, offset: i64) -> AppResult<(Vec<User>, i64)> {
        let list_query = sqlx::query_as::<_, UserRow>(
            "select * from users order by created_at desc limit $1 offset $2",
        )
        .bind(limit)
        .bind(offset);

        let rows = match &self.db {
            DbHandle::Pool(pool) => list_query.fetch_all(pool).await,
            DbHandle::Tx(tx) => list_query.fetch_all(&mut **tx.lock().await).await,
        }
        .map_err(|e| AppError::Internal(e.into()))?;

        let count_query = sqlx::query_scalar::<_, i64>("select count(*) from users");
        let total = match &self.db {
            DbHandle::Pool(pool) => count_query.fetch_one(pool).await,
            DbHandle::Tx(tx) => count_query.fetch_one(&mut **tx.lock().await).await,
        }
        .map_err(|e| AppError::Internal(e.into()))?;

        Ok((rows.into_iter().map(User::from).collect(), total))
    }
}

/// Shared insert logic for both a pool-owned transaction and a
/// `UnitOfWork`-shared one — takes the connection directly so callers
/// decide who commits.
async fn run_insert_with_event(
    conn: &mut PgConnection,
    user: &User,
    event: &UserEvent,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        insert into users (id, username, email, password_hash, created_at, updated_at)
        values ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(user.id.as_uuid())
    .bind(user.username.as_str())
    .bind(user.email.as_str())
    .bind(user.password_hash.as_str())
    .bind(user.created_at)
    .bind(user.updated_at)
    .execute(&mut *conn)
    .await?;

    let UserEvent::Registered {
        user_id,
        email,
        occurred_at,
    } = event;
    let payload = sonic_rs::json!({
        "user_id": user_id.to_string(),
        "email": email,
        "occurred_at": occurred_at.to_rfc3339(),
    })
    .to_string();

    sqlx::query(
        r#"
        insert into outbox_events (id, aggregate_id, event_type, payload, created_at)
        values ($1, $2, $3, $4::jsonb, $5)
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(user.id.as_uuid())
    .bind(event.kind())
    .bind(payload)
    .bind(*occurred_at)
    .execute(&mut *conn)
    .await?;

    sqlx::query("select pg_notify('outbox_events', $1)")
        .bind(event.kind())
        .execute(&mut *conn)
        .await?;

    Ok(())
}

fn map_sqlx_err(err: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(db_err) = &err {
        if db_err.is_unique_violation() {
            let code = match db_err.constraint() {
                Some("users_email_key") => "user/email-already-exists",
                Some("users_username_key") => "user/username-already-taken",
                _ => "resource/already-exists",
            };
            return AppError::conflict(code, "unique constraint violated");
        }
    }
    AppError::Internal(err.into())
}
