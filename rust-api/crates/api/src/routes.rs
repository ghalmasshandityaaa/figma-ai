use axum::Router;

use crate::not_found_handler::not_found_handler;
use crate::state::AppState;
use crate::{health, user};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .merge(health::routes())
        .nest("/api/v1", user::routes())
        .fallback(not_found_handler)
        .with_state(state)
}
