use kernel::AppError;
use secrecy::{ExposeSecret, SecretString};

/// User-supplied plaintext password, validated for strength and never
/// `Debug`/`Display`-able — zeroized on drop via `secrecy`.
pub struct RawPassword(SecretString);

impl RawPassword {
    pub fn parse(raw: impl Into<String>) -> Result<Self, AppError> {
        let raw = raw.into();

        if raw.len() < 8 {
            return Err(AppError::validation(
                "user/invalid-password",
                "password must be at least 8 characters",
            ));
        }
        if raw.len() > 128 {
            return Err(AppError::validation(
                "user/invalid-password",
                "password must be at most 128 characters",
            ));
        }
        let has_letter = raw.chars().any(|c| c.is_alphabetic());
        let has_digit = raw.chars().any(|c| c.is_ascii_digit());
        if !has_letter || !has_digit {
            return Err(AppError::validation(
                "user/invalid-password",
                "password must contain at least one letter and one digit",
            ));
        }

        Ok(Self(SecretString::from(raw)))
    }

    pub fn expose(&self) -> &str {
        self.0.expose_secret()
    }
}

/// Opaque Argon2id hash as persisted/loaded from storage. Construction is
/// infrastructure's job (`PasswordHasher::hash`); domain only carries it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasswordHash(String);

impl PasswordHash {
    /// Only for adapters reconstructing a `User` from a storage row.
    pub fn from_hashed(hashed: impl Into<String>) -> Self {
        Self(hashed.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
