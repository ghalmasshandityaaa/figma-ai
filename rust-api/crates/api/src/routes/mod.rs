mod health_routes;
mod user_routes;

use axum::Router;

use crate::handlers::not_found_handler;
use crate::state::AppState;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .merge(health_routes::routes())
        .nest("/api/v1", user_routes::routes())
        .fallback(not_found_handler)
        .with_state(state)
}
