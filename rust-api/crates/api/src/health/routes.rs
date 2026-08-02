use axum::routing::get;
use axum::Router;

use crate::state::AppState;

use super::handler::health_handler;

pub fn routes() -> Router<AppState> {
    Router::new().route("/healthz", get(health_handler))
}
