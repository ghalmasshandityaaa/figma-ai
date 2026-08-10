/// Records an account activation/deactivation outbox event. Swap the body
/// for a real downstream effect (e.g. revoke active sessions on
/// deactivation) — signature stays the same.
pub async fn log_user_status_change(user_id: &str, is_active: bool) -> anyhow::Result<()> {
    tracing::info!(%user_id, is_active, "user status change relayed");
    Ok(())
}
