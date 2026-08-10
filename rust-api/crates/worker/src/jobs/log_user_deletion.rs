/// Records the account-deletion outbox event. Swap the body for a real
/// downstream effect (e.g. purge PII from a search index) — signature
/// stays the same.
pub async fn log_user_deletion(user_id: &str) -> anyhow::Result<()> {
    tracing::info!(%user_id, "user deletion relayed");
    Ok(())
}
