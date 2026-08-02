use axum::http::StatusCode;

/// Liveness/readiness probe for the container orchestrator / LB.
pub async fn health_handler() -> StatusCode {
    StatusCode::OK
}
