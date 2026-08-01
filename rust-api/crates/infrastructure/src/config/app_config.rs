use serde::Deserialize;

/// Process configuration, loaded once at startup from environment variables.
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_host")]
    pub http_host: String,
    #[serde(default = "default_port")]
    pub http_port: u16,

    pub database_url: String,
    #[serde(default = "default_max_conn")]
    pub database_max_connections: u32,

    /// Base64 (standard, unpadded or padded) encoded 32-byte PASETO v4.local
    /// symmetric key. Generate with `openssl rand -base64 32`.
    pub paseto_key: String,
    #[serde(default = "default_token_ttl")]
    pub token_ttl_seconds: i64,

    #[serde(default = "default_otlp_endpoint")]
    pub otlp_endpoint: String,
    #[serde(default = "default_service_name")]
    pub service_name: String,

    /// Comma-separated allow-list. Empty means no cross-origin browser
    /// access is permitted — the safe default.
    #[serde(default)]
    pub cors_allowed_origins: String,

    #[serde(default = "default_rate_limit_per_minute")]
    pub rate_limit_per_minute: u32,
    #[serde(default = "default_rate_limit_burst")]
    pub rate_limit_burst: u32,

    #[serde(default = "default_max_body_bytes")]
    pub max_body_bytes: usize,
}

impl AppConfig {
    pub fn load() -> Result<Self, config::ConfigError> {
        config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .build()?
            .try_deserialize()
    }

    pub fn socket_addr(&self) -> String {
        format!("{}:{}", self.http_host, self.http_port)
    }

    pub fn cors_origins(&self) -> Vec<String> {
        self.cors_allowed_origins
            .split(',')
            .map(str::trim)
            .filter(|origin| !origin.is_empty())
            .map(str::to_owned)
            .collect()
    }
}

fn default_host() -> String {
    "0.0.0.0".into()
}
fn default_port() -> u16 {
    8080
}
fn default_max_conn() -> u32 {
    20
}
fn default_token_ttl() -> i64 {
    3600
}
fn default_otlp_endpoint() -> String {
    "http://tempo:4317".into()
}
fn default_service_name() -> String {
    "rust-api".into()
}
fn default_rate_limit_per_minute() -> u32 {
    120
}
fn default_rate_limit_burst() -> u32 {
    20
}
fn default_max_body_bytes() -> usize {
    1024 * 1024 // 1 MiB
}
