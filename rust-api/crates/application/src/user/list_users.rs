use std::sync::Arc;

use domain::user::UserRepository;
use kernel::AppResult;

use super::UserProfile;

pub struct ListUsersOutput {
    pub items: Vec<UserProfile>,
    pub total: i64,
}

/// One use case, one action: list users, paginated.
pub struct ListUsers {
    repository: Arc<dyn UserRepository>,
}

impl ListUsers {
    pub fn new(repository: Arc<dyn UserRepository>) -> Self {
        Self { repository }
    }

    #[tracing::instrument(name = "usecase.list_users", skip_all, fields(limit, offset))]
    pub async fn execute(&self, limit: i64, offset: i64) -> AppResult<ListUsersOutput> {
        let (users, total) = self.repository.list_page(limit, offset).await?;

        let items = users
            .into_iter()
            .map(|user| UserProfile {
                id: user.id,
                username: user.username.as_str().to_owned(),
                email: user.email.as_str().to_owned(),
                created_at: user.created_at,
            })
            .collect();

        Ok(ListUsersOutput { items, total })
    }
}
