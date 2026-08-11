use serde::Serialize;

#[derive(Serialize)]
pub struct ProductResponse {
    pub id: String,
    pub owner_id: String,
    pub name: String,
    pub description: Option<String>,
    pub price_cents: i64,
    pub currency: String,
    pub created_at: String,
}
