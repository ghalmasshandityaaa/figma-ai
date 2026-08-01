//! First-party migration CLI — no external `sqlx-cli` install required.
//! Run from the repo root (reads `./migrations`).
//!
//! ```text
//! DATABASE_URL=... cargo run -p migrator -- up
//! DATABASE_URL=... cargo run -p migrator -- down [target_version]
//! DATABASE_URL=... cargo run -p migrator -- status
//! ```

use std::path::Path;

use sqlx::migrate::Migrator;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let database_url =
        std::env::var("DATABASE_URL").map_err(|_| anyhow::anyhow!("DATABASE_URL must be set"))?;
    let command = std::env::args().nth(1).unwrap_or_else(|| "up".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;
    let migrator = Migrator::new(Path::new("./migrations")).await?;

    match command.as_str() {
        "up" => {
            migrator.run(&pool).await?;
            println!("migrations applied");
        }
        "down" => {
            let target = match std::env::args().nth(2) {
                Some(v) => v.parse::<i64>()?,
                None => previous_version(&pool).await?,
            };
            migrator.undo(&pool, target).await?;
            println!("reverted to version {target}");
        }
        "status" => print_status(&pool, &migrator).await?,
        other => anyhow::bail!("unknown command '{other}' — expected 'up', 'down', or 'status'"),
    }

    Ok(())
}

/// The version immediately before the latest applied one — i.e. "one step
/// back", which is what a bare `down` (no explicit target) should do.
async fn previous_version(pool: &PgPool) -> anyhow::Result<i64> {
    let versions: Vec<i64> = sqlx::query_scalar(
        "select version from _sqlx_migrations where success = true order by version desc limit 2",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    Ok(match versions.as_slice() {
        [_latest, previous, ..] => *previous,
        _ => 0,
    })
}

async fn print_status(pool: &PgPool, migrator: &Migrator) -> anyhow::Result<()> {
    let applied: Vec<i64> = sqlx::query_scalar(
        "select version from _sqlx_migrations where success = true order by version",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    for migration in migrator.iter().filter(|m| m.migration_type.is_up_migration()) {
        let mark = if applied.contains(&migration.version) {
            "[x]"
        } else {
            "[ ]"
        };
        println!("{mark} {:>4} {}", migration.version, migration.description);
    }

    Ok(())
}
