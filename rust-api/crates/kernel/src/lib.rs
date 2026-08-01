//! Shared, dependency-light primitives reused by every layer.
//! No web/db framework types leak in here — keeps `domain` pure.

pub mod clock;
pub mod error;
pub mod pagination;
pub mod validation_issue;

pub use clock::Clock;
pub use error::{AppError, AppResult};
pub use validation_issue::ValidationIssue;
