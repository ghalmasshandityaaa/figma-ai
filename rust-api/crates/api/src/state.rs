use std::sync::Arc;

use application::user::{GetUserProfile, ListUsers, LoginUser, RegisterUser};

/// Shared, cheaply-cloneable application state handed to every handler.
/// Holds fully-wired use cases only — no raw pool/config leaks past `main`.
#[derive(Clone)]
pub struct AppState {
    pub register_user: Arc<RegisterUser>,
    pub login_user: Arc<LoginUser>,
    pub get_user_profile: Arc<GetUserProfile>,
    pub list_users: Arc<ListUsers>,
}
