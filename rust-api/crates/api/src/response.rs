use kernel::pagination::PageMetadata;
use serde::Serialize;

/// Success envelope — the only shape any 2xx JSON body takes.
/// `metadata` is only present on paginated list endpoints.
#[derive(Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub ok: bool,
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<PageMetadata>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            ok: true,
            data,
            metadata: None,
        }
    }

    pub fn paginated(data: T, metadata: PageMetadata) -> Self {
        Self {
            ok: true,
            data,
            metadata: Some(metadata),
        }
    }
}
