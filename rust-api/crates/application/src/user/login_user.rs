use std::sync::Arc;

use domain::user::{Email, RawPassword, UserRepository};
use kernel::{AppError, AppResult};

use crate::ports::{PasswordHasher, TokenClaims, TokenIssuer};

pub struct LoginUserCommand {
    pub email: String,
    pub password: String,
}

/// One use case, one action: authenticate a user and mint an access token.
pub struct LoginUser {
    repository: Arc<dyn UserRepository>,
    hasher: Arc<dyn PasswordHasher>,
    tokens: Arc<dyn TokenIssuer>,
}

impl LoginUser {
    pub fn new(
        repository: Arc<dyn UserRepository>,
        hasher: Arc<dyn PasswordHasher>,
        tokens: Arc<dyn TokenIssuer>,
    ) -> Self {
        Self {
            repository,
            hasher,
            tokens,
        }
    }

    #[tracing::instrument(skip_all, fields(email = %cmd.email))]
    pub async fn execute(&self, cmd: LoginUserCommand) -> AppResult<TokenClaims> {
        let email = Email::parse(cmd.email)?;
        let raw_password = RawPassword::parse(cmd.password)?;

        let user = self
            .repository
            .find_by_email(&email)
            .await?
            .ok_or_else(|| AppError::unauthorized("auth/invalid-credentials", "invalid credentials"))?;

        let is_valid = self
            .hasher
            .verify(&raw_password, &user.password_hash)
            .await?;
        if !is_valid {
            return Err(AppError::unauthorized(
                "auth/invalid-credentials",
                "invalid credentials",
            ));
        }

        self.tokens.issue(user.id)
    }
}
