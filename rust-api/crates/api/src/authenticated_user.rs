use axum::extract::FromRequestParts;
use axum::http::header;
use axum::http::request::Parts;

use application::ports::TokenVerifier;
use domain::user::UserId;
use kernel::AppError;

use crate::error_response::HttpError;
use crate::state::AppState;

/// Extracts the caller's `UserId` from a `Bearer` access token. Handlers
/// opt in by naming this in their signature — there is no global auth
/// layer and no path allow-list (see ADR-0001).
pub struct AuthenticatedUser(pub UserId);

impl AuthenticatedUser {
    /// Core logic split out from the trait impl so it can be unit-tested
    /// against a stubbed `TokenVerifier` without constructing a full
    /// `AppState`.
    fn from_parts_and_verifier(
        parts: &Parts,
        verifier: &dyn TokenVerifier,
    ) -> Result<Self, HttpError> {
        let token = bearer_token(parts).ok_or_else(|| {
            AppError::unauthorized(
                "auth/missing-token",
                "missing or malformed Authorization header",
            )
        })?;

        let user_id = verifier.verify(token)?;
        Ok(AuthenticatedUser(user_id))
    }
}

#[axum::async_trait]
impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = HttpError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Self::from_parts_and_verifier(parts, state.token_verifier.as_ref())
    }
}

/// Reads `Authorization: Bearer <token>`, requiring the `Bearer` scheme
/// exactly — `None` for a missing header or any other scheme.
fn bearer_token(parts: &Parts) -> Option<&str> {
    parts
        .headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}

#[cfg(test)]
mod tests {
    use axum::http::Request;
    use kernel::AppResult;

    use super::*;

    /// Stubbed `TokenVerifier` driven by a closure — no key, no clock, no
    /// running server, per ADR-0001's testability rationale.
    struct StubVerifier<F>(F);

    impl<F> TokenVerifier for StubVerifier<F>
    where
        F: Fn(&str) -> AppResult<UserId> + Send + Sync + 'static,
    {
        fn verify(&self, token: &str) -> AppResult<UserId> {
            (self.0)(token)
        }
    }

    fn parts_with_header(value: Option<&str>) -> Parts {
        let mut builder = Request::builder();
        if let Some(value) = value {
            builder = builder.header(header::AUTHORIZATION, value);
        }
        let (parts, _) = builder.body(()).unwrap().into_parts();
        parts
    }

    #[test]
    fn valid_bearer_token_yields_the_verified_user_id() {
        let expected = UserId::new();
        let verifier = StubVerifier(move |_: &str| Ok(expected));
        let parts = parts_with_header(Some("Bearer a-valid-token"));

        let user = AuthenticatedUser::from_parts_and_verifier(&parts, &verifier)
            .ok()
            .expect("valid bearer token must be accepted");

        assert_eq!(user.0, expected);
    }

    #[test]
    fn missing_authorization_header_is_rejected_as_missing_token() {
        let verifier = StubVerifier(|_: &str| Ok(UserId::new()));
        let parts = parts_with_header(None);

        let err = AuthenticatedUser::from_parts_and_verifier(&parts, &verifier)
            .err()
            .expect("missing header must be rejected");

        assert_eq!(err.0.code(), "auth/missing-token");
    }

    #[test]
    fn non_bearer_scheme_is_rejected_as_missing_token() {
        let verifier = StubVerifier(|_: &str| Ok(UserId::new()));
        let parts = parts_with_header(Some("Basic dXNlcjpwYXNz"));

        let err = AuthenticatedUser::from_parts_and_verifier(&parts, &verifier)
            .err()
            .expect("non-Bearer scheme must be rejected");

        assert_eq!(err.0.code(), "auth/missing-token");
    }

    #[test]
    fn verifier_rejection_is_surfaced_as_invalid_token() {
        let verifier = StubVerifier(|_: &str| {
            Err(AppError::unauthorized(
                "auth/invalid-token",
                "invalid or expired token",
            ))
        });
        let parts = parts_with_header(Some("Bearer a-bad-token"));

        let err = AuthenticatedUser::from_parts_and_verifier(&parts, &verifier)
            .err()
            .expect("verifier failure must be rejected");

        assert_eq!(err.0.code(), "auth/invalid-token");
    }
}
