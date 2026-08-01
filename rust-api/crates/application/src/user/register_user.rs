use std::sync::Arc;

use chrono::Utc;
use domain::user::{Email, RawPassword, User, UserEvent, UserId, Username};
use kernel::{AppError, AppResult};

use crate::ports::{PasswordHasher, UnitOfWork};

pub struct RegisterUserCommand {
    pub username: String,
    pub email: String,
    pub password: String,
}

pub struct RegisterUserOutput {
    pub user_id: UserId,
}

/// One use case, one action: create a new user account. Nothing else lives
/// on this type — no `update`, no `delete`, no unrelated methods.
///
/// Uses the Unit-of-Work pattern: the uniqueness checks and the insert run
/// inside one transaction (`uow.execute`), so a failure after either check
/// leaves no partial row behind. The `users_email_key`/`users_username_key`
/// unique constraints remain the actual correctness guarantee under
/// concurrent registrations — the transaction gives atomicity, not
/// serializability.
pub struct RegisterUser {
    uow: Arc<dyn UnitOfWork>,
    hasher: Arc<dyn PasswordHasher>,
}

impl RegisterUser {
    pub fn new(uow: Arc<dyn UnitOfWork>, hasher: Arc<dyn PasswordHasher>) -> Self {
        Self { uow, hasher }
    }

    pub async fn execute(&self, cmd: RegisterUserCommand) -> AppResult<RegisterUserOutput> {
        let username = Username::parse(cmd.username)?;
        let email = Email::parse(cmd.email)?;
        let raw_password = RawPassword::parse(cmd.password)?;
        let password_hash = self.hasher.hash(&raw_password).await?;

        let now = Utc::now();
        let user = User::register(username, email.clone(), password_hash, now);
        let event = UserEvent::Registered {
            user_id: user.id,
            email: email.as_str().to_owned(),
            occurred_at: now,
        };
        let user_id = user.id;

        self.uow
            .execute(Box::new(move |scope| {
                Box::pin(async move {
                    let repository = scope.users();

                    if repository.find_by_email(&email).await?.is_some() {
                        return Err(AppError::conflict(
                            "user/email-already-exists",
                            "email already registered",
                        ));
                    }
                    if repository
                        .find_by_username(&user.username)
                        .await?
                        .is_some()
                    {
                        return Err(AppError::conflict(
                            "user/username-already-taken",
                            "username already taken",
                        ));
                    }

                    repository.insert_with_event(&user, &event).await
                })
            }))
            .await?;

        tracing::info!(user_id = %user_id, "user registered");
        Ok(RegisterUserOutput { user_id })
    }
}
