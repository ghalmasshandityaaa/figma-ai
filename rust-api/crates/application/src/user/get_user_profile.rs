use std::sync::Arc;

use chrono::{DateTime, Utc};
use domain::user::{UserId, UserRepository};
use kernel::{AppError, AppResult};

pub struct UserProfile {
    pub id: UserId,
    pub username: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

/// One use case, one action: read a single user's public profile.
pub struct GetUserProfile {
    repository: Arc<dyn UserRepository>,
}

impl GetUserProfile {
    pub fn new(repository: Arc<dyn UserRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, user_id: UserId) -> AppResult<UserProfile> {
        let user = self
            .repository
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::not_found("user/not-found", "user not found"))?;

        Ok(UserProfile {
            id: user.id,
            username: user.username.as_str().to_owned(),
            email: user.email.as_str().to_owned(),
            created_at: user.created_at,
        })
    }
}
