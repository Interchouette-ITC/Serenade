//! Stable contracts for adapters implemented by applications.
//!
//! Generic persistence helpers live here with zero database dependencies:
//! unit of work, pagination, shared errors, and persist-param checks.
//! Domain repository traits (catalog, cart, CMS, …) belong in the application.

pub mod error;
pub mod pagination;
pub mod persist_param;
pub mod unit_of_work;

pub use error::{PersistenceError, RepositoryError};
pub use pagination::PageRequest;
pub use persist_param::{
    persist_param_check_enabled, reject_unsafe_sql_param, reject_unsafe_sql_param_owned,
    PersistParamPolicy, PERSIST_PARAM_CHECK_DISABLE_ENV,
};
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
