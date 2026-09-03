//! Stable contracts for adapters implemented by applications.
//!
//! Repository and unit-of-work traits live here with zero database dependencies.
//! Entity types are defined in product crates; adapters implement these traits
//! with `SQLx`, `SeaORM`, or other stores.

pub mod cart;
pub mod category;
pub mod error;
pub mod order;
pub mod pagination;
pub mod product;
pub mod unit_of_work;

pub use cart::CartRepository;
pub use category::CategoryRepository;
pub use error::{PersistenceError, RepositoryError};
pub use order::OrderRepository;
pub use pagination::PageRequest;
pub use product::ProductRepository;
pub use unit_of_work::UnitOfWork;

/// Marker for entity identifiers passed into repository traits.
pub trait EntityId: Clone + PartialEq + Eq + Send + Sync + std::fmt::Debug + 'static {
    /// String form used in errors and logs.
    fn as_str(&self) -> &str;
}

impl EntityId for String {
    fn as_str(&self) -> &str {
        self.as_str()
    }
}

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
