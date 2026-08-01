/// Sends the post-registration welcome email. Swap the body for a real
/// mailer client (SES/SMTP/etc) — signature stays the same.
pub async fn send_welcome_email(email: &str) -> anyhow::Result<()> {
    tracing::info!(%email, "welcome email sent");
    Ok(())
}
