//! Pure domain layer: entities, value objects, repository ports, domain
//! events. Zero knowledge of HTTP, SQL, or JSON — infrastructure adapts to
//! this, not the other way around.

pub mod product;
pub mod user;
