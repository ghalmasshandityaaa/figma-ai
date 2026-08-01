use serde::Serialize;

#[derive(Serialize)]
pub struct ProfileResponse {
    pub id: String,
    pub username: String,
    pub email: String,
    pub created_at: String,
}
