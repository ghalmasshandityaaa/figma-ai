use std::fmt;

use kernel::AppError;

/// 3–32 chars, ASCII alphanumeric plus `_`/`-`, must start with a letter.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Username(String);

impl Username {
    pub fn parse(raw: impl AsRef<str>) -> Result<Self, AppError> {
        let value = raw.as_ref().trim().to_string();

        if !(3..=32).contains(&value.len()) {
            return Err(AppError::validation(
                "user/invalid-username",
                "username must be 3-32 characters",
            ));
        }
        let mut chars = value.chars();
        let starts_with_letter = chars.next().is_some_and(|c| c.is_ascii_alphabetic());
        let rest_valid = chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');

        if !starts_with_letter || !rest_valid {
            return Err(AppError::validation(
                "user/invalid-username",
                "username must start with a letter and contain only letters, digits, '_' or '-'",
            ));
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Username {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
