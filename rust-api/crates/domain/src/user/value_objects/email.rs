use std::fmt;

use kernel::AppError;

/// Validated, normalized (lower-cased, trimmed) email address.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Email(String);

impl Email {
    pub fn parse(raw: impl AsRef<str>) -> Result<Self, AppError> {
        let value = raw.as_ref().trim().to_lowercase();

        let (local, domain) = value
            .split_once('@')
            .ok_or_else(|| AppError::validation("user/invalid-email", "email must contain '@'"))?;

        if local.is_empty() || domain.is_empty() || !domain.contains('.') {
            return Err(AppError::validation(
                "user/invalid-email",
                "email is not a valid address",
            ));
        }
        if value.len() > 254 {
            return Err(AppError::validation(
                "user/invalid-email",
                "email exceeds 254 characters",
            ));
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
