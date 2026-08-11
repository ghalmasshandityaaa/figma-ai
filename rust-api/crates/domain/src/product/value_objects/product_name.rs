use std::fmt;

use kernel::AppError;

/// 3–120 chars after trim — the product's display name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProductName(String);

impl ProductName {
    pub fn parse(raw: impl AsRef<str>) -> Result<Self, AppError> {
        let value = raw.as_ref().trim().to_string();

        if !(3..=120).contains(&value.len()) {
            return Err(AppError::validation(
                "product/invalid-name",
                "product name must be 3-120 characters",
            ));
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProductName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
