use std::sync::Arc;

use application::ports::TokenVerifier;
use application::product::{CreateProduct, GetProduct, ListProducts};
use application::user::{
    ActivateUser, DeactivateUser, GetUserProfile, ListUsers, LoginUser, RegisterUser,
};

/// Shared, cheaply-cloneable application state handed to every handler.
/// Holds fully-wired use cases plus the `token_verifier` port used by the
/// `AuthenticatedUser` extractor — the one non-use-case dependency it
/// carries (see ADR-0001).
#[derive(Clone)]
pub struct AppState {
    pub register_user: Arc<RegisterUser>,
    pub login_user: Arc<LoginUser>,
    pub get_user_profile: Arc<GetUserProfile>,
    pub list_users: Arc<ListUsers>,
    pub activate_user: Arc<ActivateUser>,
    pub deactivate_user: Arc<DeactivateUser>,
    pub create_product: Arc<CreateProduct>,
    pub list_products: Arc<ListProducts>,
    pub get_product: Arc<GetProduct>,
    pub token_verifier: Arc<dyn TokenVerifier>,
}
