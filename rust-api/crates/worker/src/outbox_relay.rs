use std::time::Duration;

use sqlx::postgres::PgListener;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::jobs;

const BATCH_SIZE: i64 = 50;
const POLL_FALLBACK_INTERVAL: Duration = Duration::from_secs(5);

#[derive(FromRow)]
struct OutboxRow {
    id: Uuid,
    event_type: String,
    payload: String,
}

/// Relays the transactional outbox: `LISTEN` wakes it instantly on new
/// events, an interval tick is the safety net for anything missed while
/// disconnected. `FOR UPDATE SKIP LOCKED` makes multiple worker replicas
/// safe to run concurrently with no double-processing.
pub async fn run(pool: PgPool) -> anyhow::Result<()> {
    let mut listener = PgListener::connect_with(&pool).await?;
    listener.listen("outbox_events").await?;

    let mut ticker = tokio::time::interval(POLL_FALLBACK_INTERVAL);

    loop {
        tokio::select! {
            _ = listener.recv() => {}
            _ = ticker.tick() => {}
        }

        if let Err(err) = drain_batch(&pool).await {
            tracing::error!(error = %err, "failed draining outbox batch");
        }
    }
}

async fn drain_batch(pool: &PgPool) -> anyhow::Result<()> {
    loop {
        let mut tx = pool.begin().await?;

        let rows: Vec<OutboxRow> = sqlx::query_as(
            r#"
            select id, event_type, payload::text as payload
            from outbox_events
            where processed_at is null
            order by created_at
            limit $1
            for update skip locked
            "#,
        )
        .bind(BATCH_SIZE)
        .fetch_all(&mut *tx)
        .await?;

        if rows.is_empty() {
            tx.commit().await?;
            return Ok(());
        }

        for row in &rows {
            if let Err(err) = jobs::dispatch(&row.event_type, row.payload.as_bytes()).await {
                tracing::error!(event_id = %row.id, error = %err, "job handler failed, will retry");
                continue;
            }

            sqlx::query("update outbox_events set processed_at = now() where id = $1")
                .bind(row.id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
    }
}
