use chrono::{DateTime, Utc};

use super::value_objects::UserId;

/// Domain events persisted to the outbox table in the same transaction as
/// the aggregate write, then relayed by the worker (LISTEN/NOTIFY).
#[derive(Debug, Clone)]
pub enum UserEvent {
    Registered {
        user_id: UserId,
        email: String,
        occurred_at: DateTime<Utc>,
    },
}

impl UserEvent {
    pub fn kind(&self) -> &'static str {
        match self {
            UserEvent::Registered { .. } => "user.registered",
        }
    }
}
