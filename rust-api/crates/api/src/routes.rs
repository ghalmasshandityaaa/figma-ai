use axum::Router;

use crate::not_found_handler::not_found_handler;
use crate::state::AppState;
use crate::{health, product, user};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .merge(health::routes())
        .nest("/api/v1", user::routes().merge(product::routes()))
        .fallback(not_found_handler)
        .with_state(state)
}
