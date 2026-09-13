//! Admin errors.

/// Failure while configuring or running admin CRUD.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum AdminError {
    /// Two resources share the same name.
    #[error("admin resource `{name}` is already registered")]
    DuplicateResource {
        /// Resource name.
        name: String,
    },
    /// App handler reported a missing entity.
    #[error("admin entity `{id}` not found")]
    NotFound {
        /// Entity id.
        id: String,
    },
    /// App handler / repository failure.
    #[error("admin persist: {message}")]
    Persist {
        /// Reason text.
        message: String,
    },
}
