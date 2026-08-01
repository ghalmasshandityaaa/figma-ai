use serde::Serialize;

#[derive(Serialize)]
pub struct RegisterResponse {
    pub user_id: String,
}
