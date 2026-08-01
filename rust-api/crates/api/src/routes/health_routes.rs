use axum::routing::get;
use axum::Router;

use crate::handlers::health_handler;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/healthz", get(health_handler))
}
