use thiserror::Error;

use crate::validation_issue::ValidationIssue;

/// Transport-agnostic error taxonomy. Every variant carries a stable,
/// Firebase-style `domain/reason` code — the only thing ever returned to
/// API clients. Human-readable `message` is for logs/tracing only.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("{message}")]
    Validation {
        code: &'static str,
        message: String,
        issues: Vec<ValidationIssue>,
    },

    #[error("{message}")]
    NotFound { code: &'static str, message: String },

    #[error("{message}")]
    Conflict { code: &'static str, message: String },

    #[error("{message}")]
    Unauthorized { code: &'static str, message: String },

    #[error("{message}")]
    Forbidden { code: &'static str, message: String },

    #[error("too many requests")]
    RateLimited,

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    pub fn validation(code: &'static str, message: impl Into<String>) -> Self {
        Self::Validation {
            code,
            message: message.into(),
            issues: Vec::new(),
        }
    }

    pub fn validation_with_issues(
        code: &'static str,
        message: impl Into<String>,
        issues: Vec<ValidationIssue>,
    ) -> Self {
        Self::Validation {
            code,
            message: message.into(),
            issues,
        }
    }

    pub fn not_found(code: &'static str, message: impl Into<String>) -> Self {
        Self::NotFound {
            code,
            message: message.into(),
        }
    }

    pub fn conflict(code: &'static str, message: impl Into<String>) -> Self {
        Self::Conflict {
            code,
            message: message.into(),
        }
    }

    pub fn unauthorized(code: &'static str, message: impl Into<String>) -> Self {
        Self::Unauthorized {
            code,
            message: message.into(),
        }
    }

    pub fn forbidden(code: &'static str, message: impl Into<String>) -> Self {
        Self::Forbidden {
            code,
            message: message.into(),
        }
    }

    /// Stable machine-readable code — the only error detail exposed to
    /// clients. `internal/server-error` always hides the real cause.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Validation { code, .. } => code,
            Self::NotFound { code, .. } => code,
            Self::Conflict { code, .. } => code,
            Self::Unauthorized { code, .. } => code,
            Self::Forbidden { code, .. } => code,
            Self::RateLimited => "rate-limit/exceeded",
            Self::Internal(_) => "internal/server-error",
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;
