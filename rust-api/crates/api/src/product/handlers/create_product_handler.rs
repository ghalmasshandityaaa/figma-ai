use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use garde::Validate;

use application::product::CreateProductCommand;

use crate::authenticated_user::AuthenticatedUser;
use crate::error_response::HttpError;
use crate::json::Sonic;
use crate::product::dto::{CreateProductRequest, ProductResponse};
use crate::response::ApiResponse;
use crate::state::AppState;
use crate::validation::to_app_error;

/// Requires a valid bearer token (see ADR-0001): `owner_id` is the verified
/// caller's `UserId`, never a client-supplied field.
#[tracing::instrument(name = "handler.create_product", skip_all, fields(owner_id = %owner_id, name = %body.name))]
pub async fn create_product_handler(
    State(state): State<AppState>,
    AuthenticatedUser(owner_id): AuthenticatedUser,
    Sonic(body): Sonic<CreateProductRequest>,
) -> Result<Response, HttpError> {
    body.validate().map_err(to_app_error)?;

    let output = state
        .create_product
        .execute(CreateProductCommand {
            owner_id,
            name: body.name,
            description: body.description,
            price_cents: body.price_cents,
            currency: body.currency,
        })
        .await?;

    let response = ProductResponse {
        id: output.id.to_string(),
        owner_id: output.owner_id.to_string(),
        name: output.name,
        description: output.description,
        price_cents: output.price_cents,
        currency: output.currency,
        created_at: output.created_at.to_rfc3339(),
    };
    Ok((StatusCode::CREATED, Sonic(ApiResponse::ok(response))).into_response())
}
