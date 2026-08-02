use axum::extract::State;
use garde::Validate;

use application::user::LoginUserCommand;

use crate::dto::{LoginRequest, LoginResponse};
use crate::error_response::HttpError;
use crate::json::Sonic;
use crate::response::ApiResponse;
use crate::state::AppState;
use crate::validation::to_app_error;

#[tracing::instrument(name = "handler.login", skip_all, fields(email = %body.email))]
pub async fn login_handler(
    State(state): State<AppState>,
    Sonic(body): Sonic<LoginRequest>,
) -> Result<Sonic<ApiResponse<LoginResponse>>, HttpError> {
    body.validate().map_err(to_app_error)?;

    let claims = state
        .login_user
        .execute(LoginUserCommand {
            email: body.email,
            password: body.password,
        })
        .await?;

    Ok(Sonic(ApiResponse::ok(LoginResponse {
        access_token: claims.access_token,
        token_type: "Bearer",
        expires_in: claims.expires_in_seconds,
    })))
}
