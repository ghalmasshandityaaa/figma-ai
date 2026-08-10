use std::sync::Arc;

use chrono::Utc;
use domain::user::{UserEvent, UserId, UserRepository};
use kernel::{AppError, AppResult};

/// One use case, one action: reinstate a suspended account. Rejects a
/// no-op activation up front so the outbox never records a spurious
/// `user.activated` event for a user that was already active.
pub struct ActivateUser {
    repository: Arc<dyn UserRepository>,
}

impl ActivateUser {
    pub fn new(repository: Arc<dyn UserRepository>) -> Self {
        Self { repository }
    }

    #[tracing::instrument(name = "usecase.activate_user", skip_all, fields(user_id = %user_id))]
    pub async fn execute(&self, user_id: UserId) -> AppResult<()> {
        let user = self
            .repository
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::not_found("user/not-found", "user not found"))?;

        if user.is_active {
            return Err(AppError::conflict(
                "user/already-active",
                "user is already active",
            ));
        }

        let event = UserEvent::Activated {
            user_id,
            occurred_at: Utc::now(),
        };

        let changed = self
            .repository
            .set_active_with_event(user_id, true, &event)
            .await?;
        if !changed {
            return Err(AppError::not_found("user/not-found", "user not found"));
        }

        tracing::info!(user_id = %user_id, "user activated");
        Ok(())
    }
}
