use serde::Serialize;

/// Shared response shape for both the activate and deactivate endpoints —
/// only the `is_active` value differs.
#[derive(Serialize)]
pub struct UserStatusResponse {
    pub id: String,
    pub is_active: bool,
}
