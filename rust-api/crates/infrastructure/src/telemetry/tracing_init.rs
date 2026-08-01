use opentelemetry::trace::TracerProvider as _;
use opentelemetry::KeyValue;
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Dropping this shuts the OTLP batch exporter down and flushes the
/// non-blocking log writer — hold it for the lifetime of `main`.
pub struct TelemetryGuard {
    provider: SdkTracerProvider,
    _writer_guard: WorkerGuard,
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        if let Err(err) = self.provider.shutdown() {
            eprintln!("otel tracer shutdown failed: {err}");
        }
    }
}

/// Wires `tracing` spans into OTLP (exported to Tempo) plus JSON logs on
/// stdout — no Prometheus involved. Logging is non-blocking: `fmt::layer`
/// writes go through a channel to a dedicated I/O thread
/// (`tracing_appender::non_blocking`), so a slow stdout/pipe never stalls
/// the async executor on the hot request path.
pub fn init_tracing(service_name: &str, otlp_endpoint: &str) -> anyhow::Result<TelemetryGuard> {
    let exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(otlp_endpoint)
        .build()?;

    let resource = Resource::builder()
        .with_attribute(KeyValue::new("service.name", service_name.to_string()))
        .build();

    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build();

    let tracer = provider.tracer(service_name.to_string());
    opentelemetry::global::set_tracer_provider(provider.clone());

    let (non_blocking_writer, writer_guard) = tracing_appender::non_blocking(std::io::stdout());

    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    // Every #[instrument]-created span (handler/use-case/repository/...) gets an
    // automatic "new" log line on entry and a "close" log line on drop, the latter
    // carrying `time.busy`/`time.idle` — a start/end pair per layer for free, with
    // per-layer latency, correlated by the shared trace/span context. NEW+CLOSE
    // (not ENTER+EXIT) fire exactly once per span even for a `.await`ed async fn
    // that yields many times, so this doesn't spam logs.
    let fmt_layer = tracing_subscriber::fmt::layer()
        .json()
        .flatten_event(true)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_writer(non_blocking_writer);
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(otel_layer)
        .try_init()?;

    Ok(TelemetryGuard {
        provider,
        _writer_guard: writer_guard,
    })
}
