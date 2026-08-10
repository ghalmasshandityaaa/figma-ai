mod log_user_deletion;
mod send_welcome_email;

use sonic_rs::JsonValueTrait;

use log_user_deletion::log_user_deletion;
use send_welcome_email::send_welcome_email;

/// Dispatches an outbox event to its handler by `event_type`. One `match`
/// arm per event — adding a job never touches unrelated handlers.
pub async fn dispatch(event_type: &str, payload: &[u8]) -> anyhow::Result<()> {
    match event_type {
        "user.registered" => {
            let value: sonic_rs::Value = sonic_rs::from_slice(payload)?;
            let email = value["email"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("outbox payload missing `email`"))?;
            send_welcome_email(email).await
        }
        "user.deleted" => {
            let value: sonic_rs::Value = sonic_rs::from_slice(payload)?;
            let user_id = value["user_id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("outbox payload missing `user_id`"))?;
            log_user_deletion(user_id).await
        }
        other => {
            tracing::warn!(event_type = other, "no handler registered for event type");
            Ok(())
        }
    }
}
