use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// Non-blocking connection pool; `max_connections` should track
/// `db.max_connections / instance_count`, not thread count.
pub async fn build_pool(database_url: &str, max_connections: u32) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .test_before_acquire(true)
        .connect(database_url)
        .await?;

    sqlx::migrate!("../../migrations").run(&pool).await?;

    Ok(pool)
}
