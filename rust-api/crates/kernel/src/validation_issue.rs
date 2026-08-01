use serde::Serialize;

/// One field-level validation failure — safe to expose to API clients
/// (unlike internal error messages, which are never surfaced).
#[derive(Debug, Clone, Serialize)]
pub struct ValidationIssue {
    pub field: String,
    pub message: String,
}

impl ValidationIssue {
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
        }
    }
}
