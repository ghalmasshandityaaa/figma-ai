use axum::http::HeaderName;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};

pub const REQUEST_ID_HEADER: &str = "x-request-id";

pub fn set_request_id_layer() -> SetRequestIdLayer<MakeRequestUuid> {
    SetRequestIdLayer::new(HeaderName::from_static(REQUEST_ID_HEADER), MakeRequestUuid)
}

pub fn propagate_request_id_layer() -> PropagateRequestIdLayer {
    PropagateRequestIdLayer::new(HeaderName::from_static(REQUEST_ID_HEADER))
}
