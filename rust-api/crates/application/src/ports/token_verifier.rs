use domain::user::UserId;
use kernel::AppResult;

/// Verifies opaque access tokens minted by `TokenIssuer`, returning the
/// `UserId` they were issued for.
pub trait TokenVerifier: Send + Sync + 'static {
    fn verify(&self, token: &str) -> AppResult<UserId>;
}
