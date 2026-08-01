//! Application layer: one file per use case, one `execute` per use case.
//! Orchestrates domain objects through ports (`PasswordHasher`,
//! `TokenIssuer`) — never touches SQL or HTTP directly.

pub mod ports;
pub mod user;
