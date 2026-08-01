use axum::extract::{Query, State};
use kernel::pagination::{PageMetadata, PageRequest};

use crate::dto::ProfileResponse;
use crate::error_response::HttpError;
use crate::json::Sonic;
use crate::response::ApiResponse;
use crate::state::AppState;

#[tracing::instrument(skip_all, fields(page = page.page, page_size = page.page_size))]
pub async fn list_users_handler(
    State(state): State<AppState>,
    Query(page): Query<PageRequest>,
) -> Result<Sonic<ApiResponse<Vec<ProfileResponse>>>, HttpError> {
    let output = state.list_users.execute(page.limit(), page.offset()).await?;

    let items = output
        .items
        .into_iter()
        .map(|profile| ProfileResponse {
            id: profile.id.to_string(),
            username: profile.username,
            email: profile.email,
            created_at: profile.created_at.to_rfc3339(),
        })
        .collect();

    let metadata = PageMetadata::new(page, output.total);
    Ok(Sonic(ApiResponse::paginated(items, metadata)))
}
