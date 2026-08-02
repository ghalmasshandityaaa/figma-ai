use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use garde::Validate;

use application::user::RegisterUserCommand;

use crate::user::dto::{RegisterRequest, RegisterResponse};
use crate::error_response::HttpError;
use crate::json::Sonic;
use crate::response::ApiResponse;
use crate::state::AppState;
use crate::validation::to_app_error;

#[tracing::instrument(name = "handler.register", skip_all, fields(email = %body.email))]
pub async fn register_handler(
    State(state): State<AppState>,
    Sonic(body): Sonic<RegisterRequest>,
) -> Result<Response, HttpError> {
    body.validate().map_err(to_app_error)?;

    let output = state
        .register_user
        .execute(RegisterUserCommand {
            username: body.username,
            email: body.email,
            password: body.password,
        })
        .await?;

    let response = RegisterResponse {
        user_id: output.user_id.to_string(),
    };
    Ok((StatusCode::CREATED, Sonic(ApiResponse::ok(response))).into_response())
}
