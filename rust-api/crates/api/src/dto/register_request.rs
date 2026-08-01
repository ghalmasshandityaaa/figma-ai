use garde::Validate;
use serde::Deserialize;

/// Shape-level validation (garde) runs first; deeper domain invariants
/// (VOs) run again in the use case — cheap, and keeps domain rules in one
/// place instead of duplicated across every caller.
#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[garde(length(min = 3, max = 32))]
    pub username: String,
    #[garde(email)]
    pub email: String,
    #[garde(length(min = 8, max = 128))]
    pub password: String,
}
