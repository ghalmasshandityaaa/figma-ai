use application::ports::PasswordHasher;
use argon2::password_hash::{rand_core::OsRng, PasswordHash as Argon2Hash, PasswordHasher as _, PasswordVerifier, SaltString};
use argon2::Argon2;
use async_trait::async_trait;
use domain::user::{PasswordHash, RawPassword};
use kernel::AppError::Internal;
use kernel::AppResult;

#[derive(Default, Clone, Copy)]
pub struct Argon2PasswordHasher;

#[async_trait]
impl PasswordHasher for Argon2PasswordHasher {
    #[tracing::instrument(name = "password_hasher.hash", skip_all)]
    async fn hash(&self, raw: &RawPassword) -> AppResult<PasswordHash> {
        let plain = raw.expose().to_owned();
        // CPU-bound: keep it off the async reactor.
        let hashed = tokio::task::spawn_blocking(move || {
            let salt = SaltString::generate(&mut OsRng);
            Argon2::default()
                .hash_password(plain.as_bytes(), &salt)
                .map(|h| h.to_string())
        })
        .await
        .map_err(|e| Internal(e.into()))?
        .map_err(|e| Internal(anyhow::anyhow!(e.to_string())))?;

        Ok(PasswordHash::from_hashed(hashed))
    }

    #[tracing::instrument(name = "password_hasher.verify", skip_all)]
    async fn verify(&self, raw: &RawPassword, hash: &PasswordHash) -> AppResult<bool> {
        let plain = raw.expose().to_owned();
        let hash_str = hash.as_str().to_owned();

        let is_valid = tokio::task::spawn_blocking(move || {
            let parsed = Argon2Hash::new(&hash_str)?;
            Ok::<bool, argon2::password_hash::Error>(
                Argon2::default()
                    .verify_password(plain.as_bytes(), &parsed)
                    .is_ok(),
            )
        })
        .await
        .map_err(|e| Internal(e.into()))?
        .map_err(|e| Internal(anyhow::anyhow!(e.to_string())))?;

        Ok(is_valid)
    }
}
