use std::sync::Arc;

use chrono::Utc;
use domain::user::{UserEvent, UserId, UserRepository};
use kernel::{AppError, AppResult};

/// One use case, one action: suspend an account without deleting it —
/// blocks future logins (`LoginUser` checks `is_active`) while keeping the
/// row and its history intact. Rejects a no-op deactivation up front so the
/// outbox never records a spurious `user.deactivated` event.
pub struct DeactivateUser {
    repository: Arc<dyn UserRepository>,
}

impl DeactivateUser {
    pub fn new(repository: Arc<dyn UserRepository>) -> Self {
        Self { repository }
    }

    #[tracing::instrument(name = "usecase.deactivate_user", skip_all, fields(user_id = %user_id))]
    pub async fn execute(&self, user_id: UserId) -> AppResult<()> {
        let user = self
            .repository
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::not_found("user/not-found", "user not found"))?;

        if !user.is_active {
            return Err(AppError::conflict(
                "user/already-inactive",
                "user is already inactive",
            ));
        }

        let event = UserEvent::Deactivated {
            user_id,
            occurred_at: Utc::now(),
        };

        let changed = self
            .repository
            .set_active_with_event(user_id, false, &event)
            .await?;
        if !changed {
            return Err(AppError::not_found("user/not-found", "user not found"));
        }

        tracing::info!(user_id = %user_id, "user deactivated");
        Ok(())
    }
}
