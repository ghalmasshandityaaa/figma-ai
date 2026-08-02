mod not_found_handler;

pub mod health;
pub mod user;

pub use health::health_handler;
pub use not_found_handler::not_found_handler;
pub use user::{get_profile_handler, list_users_handler, login_handler, register_handler};
