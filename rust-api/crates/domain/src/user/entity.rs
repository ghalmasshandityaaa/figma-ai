use chrono::{DateTime, Utc};

use super::value_objects::{Email, PasswordHash, UserId, Username};

/// Aggregate root for the `user` bounded context.
#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub username: Username,
    pub email: Email,
    pub password_hash: PasswordHash,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// Factory for a brand-new registration — invariants already hold
    /// because every field is a validated value object.
    pub fn register(
        username: Username,
        email: Email,
        password_hash: PasswordHash,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: UserId::new(),
            username,
            email,
            password_hash,
            created_at: now,
            updated_at: now,
        }
    }
}
