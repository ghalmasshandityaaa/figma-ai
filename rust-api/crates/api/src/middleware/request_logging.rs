use std::time::Instant;

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use tracing::Instrument;

use super::request_id::REQUEST_ID_HEADER;

/// Full request lifecycle logging: one span per request (exported to Tempo
/// via `tracing-opentelemetry`), with an explicit "started" log on entry
/// and a "completed" log after `next` returns — so every request's
/// open/close is visible in both logs and traces, correlated by
/// `request_id`. Panicking requests are logged separately by the panic
/// handler, since `next.run` never returns in that case.
pub async fn request_logging(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_owned();
    let request_id = req
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
        .to_owned();

    // `otel.name` is a tracing-opentelemetry convention: when present, it
    // overrides the span's exported OTEL name (the static "http_request"
    // stays as-is in JSON logs/`target`). Tempo's trace list "Name" column
    // shows the root span's name, so this turns every row into e.g.
    // "GET /api/v1/users" instead of an indistinguishable "http_request" —
    // no per-query TraceQL `select()` needed to tell requests apart.
    let route = format!("{method} {path}");
    let span = tracing::info_span!(
        "http_request",
        otel.name = %route,
        request_id = %request_id,
        http.method = %method,
        http.path = %path,
        http.status_code = tracing::field::Empty,
        latency_ms = tracing::field::Empty,
    );

    async move {
        tracing::info!("request started");
        let start = Instant::now();

        let response = next.run(req).await;

        let latency_ms = start.elapsed().as_millis() as u64;
        let status = response.status().as_u16();
        tracing::Span::current().record("http.status_code", status);
        tracing::Span::current().record("latency_ms", latency_ms);

        tracing::info!(status, latency_ms, "request completed");
        response
    }
    .instrument(span)
    .await
}
