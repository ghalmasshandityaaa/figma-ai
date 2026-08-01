//! Adapters that implement domain/application ports against concrete
//! technology: Postgres, Argon2, JWT, OpenTelemetry.

pub mod config;
pub mod postgres;
pub mod security;
pub mod telemetry;
