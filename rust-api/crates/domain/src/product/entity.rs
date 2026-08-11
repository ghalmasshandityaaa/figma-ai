use chrono::{DateTime, Utc};
use kernel::{AppError, AppResult};

use super::value_objects::{Money, ProductId, ProductName};
use crate::user::UserId;

/// Aggregate root for the `product` bounded context.
#[derive(Debug, Clone)]
pub struct Product {
    pub id: ProductId,
    pub owner_id: UserId,
    pub name: ProductName,
    pub description: Option<String>,
    pub money: Money,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Product {
    /// Factory for a brand-new listing — invariants already hold for
    /// `name`/`money` because they're validated value objects; `description`
    /// is re-validated here since it stays a plain `Option<String>` on the
    /// aggregate (not worth a dedicated value object for one bounded field).
    pub fn create(
        owner_id: UserId,
        name: ProductName,
        description: Option<String>,
        money: Money,
        now: DateTime<Utc>,
    ) -> AppResult<Self> {
        Ok(Self {
            id: ProductId::new(),
            owner_id,
            name,
            description: normalize_description(description)?,
            money,
            created_at: now,
            updated_at: now,
        })
    }
}

/// Trims and blanks out an empty description; rejects one over 2000 chars.
fn normalize_description(description: Option<String>) -> AppResult<Option<String>> {
    let Some(raw) = description else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > 2000 {
        return Err(AppError::validation(
            "product/invalid-description",
            "description must be at most 2000 characters",
        ));
    }
    Ok(Some(trimmed.to_owned()))
}
