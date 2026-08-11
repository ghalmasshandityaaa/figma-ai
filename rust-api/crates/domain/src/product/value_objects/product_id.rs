use std::fmt;

use uuid::Uuid;

/// Opaque identity — UUIDv7 so ids stay roughly time-ordered (index-friendly).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProductId(Uuid);

impl ProductId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for ProductId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ProductId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
