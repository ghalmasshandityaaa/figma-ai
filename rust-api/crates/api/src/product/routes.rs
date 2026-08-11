use axum::routing::get;
use axum::Router;

use crate::state::AppState;

use super::handlers::{
    create_product_handler, delete_product_handler, get_product_handler, list_products_handler,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/products",
            get(list_products_handler).post(create_product_handler),
        )
        .route(
            "/products/:id",
            get(get_product_handler).delete(delete_product_handler),
        )
}
