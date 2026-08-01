use domain::user::UserId;
use kernel::AppResult;

pub struct TokenClaims {
    pub access_token: String,
    pub expires_in_seconds: i64,
}

/// Issues opaque access tokens (JWT under the hood) for an authenticated user.
pub trait TokenIssuer: Send + Sync + 'static {
    fn issue(&self, user_id: UserId) -> AppResult<TokenClaims>;
}
