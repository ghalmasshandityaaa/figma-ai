use std::convert::TryFrom;

use application::ports::{TokenClaims, TokenIssuer, TokenVerifier};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::{Duration, Utc};
use domain::user::UserId;
use kernel::{AppError, AppResult};
use rusty_paseto::prelude::*;
use uuid::Uuid;

/// PASETO v4.local (XChaCha20-Poly1305, symmetric) access tokens — no
/// algorithm-confusion surface like JWT's `alg: none`/RS-vs-HS attacks,
/// since the version+purpose is fixed at compile time by the type itself.
pub struct PasetoTokenIssuer {
    key: PasetoSymmetricKey<V4, Local>,
    ttl_seconds: i64,
}

impl PasetoTokenIssuer {
    /// `key_base64` must decode to exactly 32 bytes
    /// (e.g. `openssl rand -base64 32`).
    pub fn new(key_base64: &str, ttl_seconds: i64) -> anyhow::Result<Self> {
        let raw = STANDARD
            .decode(key_base64.trim())
            .map_err(|e| anyhow::anyhow!("PASETO_KEY must be valid base64: {e}"))?;
        let key = Key::<32>::try_from(raw.as_slice())
            .map_err(|_| anyhow::anyhow!("PASETO_KEY must decode to exactly 32 bytes"))?;

        Ok(Self {
            key: PasetoSymmetricKey::<V4, Local>::from(key),
            ttl_seconds,
        })
    }
}

impl TokenIssuer for PasetoTokenIssuer {
    #[tracing::instrument(name = "token_issuer.issue", skip_all, fields(user_id = %user_id))]
    fn issue(&self, user_id: UserId) -> AppResult<TokenClaims> {
        let expires_at = (Utc::now() + Duration::seconds(self.ttl_seconds)).to_rfc3339();

        let expiration_claim = ExpirationClaim::try_from(expires_at)
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;

        let subject = user_id.to_string();
        let access_token = PasetoBuilder::<V4, Local>::default()
            .set_claim(expiration_claim)
            .set_claim(SubjectClaim::from(subject.as_str()))
            .build(&self.key)
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;

        Ok(TokenClaims {
            access_token,
            expires_in_seconds: self.ttl_seconds,
        })
    }
}

impl TokenVerifier for PasetoTokenIssuer {
    /// Every failure mode — bad ciphertext, wrong key, expired, unparseable
    /// subject — maps to the same `auth/invalid-token` code, deliberately
    /// not distinguishing expired from forged (mirrors `LoginUser` giving
    /// one code for both a wrong email and a wrong password).
    #[tracing::instrument(name = "token_verifier.verify", skip_all)]
    fn verify(&self, token: &str) -> AppResult<UserId> {
        // `default()` validates ExpirationClaim (and NotBeforeClaim, unset
        // here) automatically before returning the claim set.
        let claims = PasetoParser::<V4, Local>::default()
            .parse(token, &self.key)
            .map_err(|_| {
                AppError::unauthorized("auth/invalid-token", "invalid or expired token")
            })?;

        let subject = claims["sub"].as_str().ok_or_else(|| {
            AppError::unauthorized("auth/invalid-token", "invalid or expired token")
        })?;

        let user_id = Uuid::parse_str(subject).map_err(|_| {
            AppError::unauthorized("auth/invalid-token", "invalid or expired token")
        })?;

        Ok(UserId::from_uuid(user_id))
    }
}
