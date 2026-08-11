use garde::Validate;
use serde::Deserialize;

/// Shape-level validation (garde) runs first; deeper domain invariants
/// (`ProductName`/`Money`) run again in the use case — same two-layer
/// pattern as `RegisterRequest`. No `owner_id` field — the caller's
/// identity comes from `AuthenticatedUser`, never the request body.
#[derive(Debug, Deserialize, Validate)]
pub struct CreateProductRequest {
    #[garde(length(min = 3, max = 120))]
    pub name: String,
    #[garde(length(max = 2000))]
    pub description: Option<String>,
    #[garde(range(min = 0))]
    pub price_cents: i64,
    #[garde(length(min = 3, max = 3), custom(is_uppercase_currency))]
    pub currency: String,
}

fn is_uppercase_currency(value: &str, _ctx: &()) -> garde::Result {
    if value.chars().all(|c| c.is_ascii_uppercase()) {
        Ok(())
    } else {
        Err(garde::Error::new(
            "currency must be a 3-letter uppercase ISO-4217 code",
        ))
    }
}
