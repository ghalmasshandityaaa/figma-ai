use serde::Serialize;

#[derive(Serialize)]
pub struct DeleteResponse {
    pub id: String,
}
