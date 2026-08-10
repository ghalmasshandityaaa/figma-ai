use std::sync::Arc;

use chrono::Utc;
use domain::user::{UserEvent, UserId, UserRepository};
use kernel::{AppError, AppResult};

/// One use case, one action: permanently remove a user account. The
/// repository writes the deletion and its `user.deleted` outbox event in
/// one transaction (`delete_with_event`) — this use case owns only the
/// not-found mapping.
pub struct DeleteUser {
    repository: Arc<dyn UserRepository>,
}

impl DeleteUser {
    pub fn new(repository: Arc<dyn UserRepository>) -> Self {
        Self { repository }
    }

    #[tracing::instrument(name = "usecase.delete_user", skip_all, fields(user_id = %user_id))]
    pub async fn execute(&self, user_id: UserId) -> AppResult<()> {
        let event = UserEvent::Deleted {
            user_id,
            occurred_at: Utc::now(),
        };

        let deleted = self.repository.delete_with_event(user_id, &event).await?;
        if !deleted {
            return Err(AppError::not_found("user/not-found", "user not found"));
        }

        tracing::info!(user_id = %user_id, "user deleted");
        Ok(())
    }
}
