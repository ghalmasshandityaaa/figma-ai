// PLAYG-005 (blocked on this ticket) is the first handler to name
// `AuthenticatedUser` in its signature — until then the extractor is
// intentionally unused, not dead.
#[allow(dead_code)]
mod authenticated_user;
mod error_response;
mod health;
mod json;
mod middleware;
mod not_found_handler;
mod response;
mod routes;
mod state;
mod user;
mod validation;

use std::net::SocketAddr;
use std::sync::Arc;

use application::user::{
    ActivateUser, DeactivateUser, GetUserProfile, ListUsers, LoginUser, RegisterUser,
};
use infrastructure::config::AppConfig;
use infrastructure::postgres::{build_pool, PgUnitOfWork, PgUserRepository};
use infrastructure::security::{Argon2PasswordHasher, PasetoTokenIssuer};
use infrastructure::telemetry::init_tracing;

use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Only present in local dev; docker/production set real env vars and
    // won't have a `.env` file, so a missing file is silently ignored.
    dotenvy::dotenv().ok();

    let config = AppConfig::load()?;
    let _telemetry = init_tracing(&config.service_name, &config.otlp_endpoint)?;

    let pool = build_pool(&config.database_url, config.database_max_connections).await?;
    let repository = Arc::new(PgUserRepository::new(pool.clone()));
    let unit_of_work = Arc::new(PgUnitOfWork::new(pool));
    let hasher = Arc::new(Argon2PasswordHasher);
    let tokens = Arc::new(PasetoTokenIssuer::new(
        &config.paseto_key,
        config.token_ttl_seconds,
    )?);

    let state = AppState {
        register_user: Arc::new(RegisterUser::new(unit_of_work, hasher.clone())),
        login_user: Arc::new(LoginUser::new(repository.clone(), hasher, tokens.clone())),
        get_user_profile: Arc::new(GetUserProfile::new(repository.clone())),
        list_users: Arc::new(ListUsers::new(repository.clone())),
        activate_user: Arc::new(ActivateUser::new(repository.clone())),
        deactivate_user: Arc::new(DeactivateUser::new(repository)),
        token_verifier: tokens,
    };

    let router = middleware::apply(routes::build_router(state), &config);
    let addr: SocketAddr = config.socket_addr().parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!(%addr, "api listening");

    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install ctrl+c handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received");
}
