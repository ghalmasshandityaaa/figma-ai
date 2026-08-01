mod jobs;
mod outbox_relay;

use infrastructure::config::AppConfig;
use infrastructure::postgres::build_pool;
use infrastructure::telemetry::init_tracing;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let config = AppConfig::load()?;
    let _telemetry = init_tracing(&format!("{}-worker", config.service_name), &config.otlp_endpoint)?;

    let pool = build_pool(&config.database_url, config.database_max_connections).await?;

    tracing::info!("outbox worker started");
    outbox_relay::run(pool).await
}
