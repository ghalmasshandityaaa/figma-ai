use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

use super::handlers::{
    activate_user_handler, deactivate_user_handler, delete_user_handler, get_profile_handler,
    list_users_handler, login_handler, register_handler,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/users", post(register_handler).get(list_users_handler))
        .route(
            "/users/:id",
            get(get_profile_handler).delete(delete_user_handler),
        )
        .route("/users/:id/activate", post(activate_user_handler))
        .route("/users/:id/deactivate", post(deactivate_user_handler))
        .route("/auth/login", post(login_handler))
}
