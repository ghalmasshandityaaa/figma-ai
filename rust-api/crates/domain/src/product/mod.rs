pub mod entity;
pub mod repository;
pub mod value_objects;

pub use entity::Product;
pub use repository::ProductRepository;
pub use value_objects::{Money, ProductId, ProductName};
