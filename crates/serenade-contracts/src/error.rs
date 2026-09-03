//! Persistence errors returned by repository and unit-of-work adapters.

/// Supertrait for errors surfaced by repository implementations.
pub trait RepositoryError: std::error::Error + Send + Sync + 'static {}

/// ORM-agnostic persistence failure modes.
#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    /// Row or aggregate missing for the given identifier.
    #[error("not found: {entity} id={id}")]
    NotFound {
        /// Aggregate name (for example `product`).
        entity: &'static str,
        /// Identifier string.
        id: String,
    },
    /// Unique constraint or optimistic lock conflict.
    #[error("conflict on {constraint}")]
    Conflict {
        /// Constraint or field name.
        constraint: &'static str,
    },
    /// Invalid input before touching storage.
    #[error("invalid input: {message}")]
    InvalidInput {
        /// Human-readable reason.
        message: String,
    },
    /// Unexpected adapter or infrastructure failure.
    #[error("internal persistence error: {message}")]
    Internal {
        /// Human-readable reason.
        message: String,
    },
}

impl RepositoryError for PersistenceError {}
