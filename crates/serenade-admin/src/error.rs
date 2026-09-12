//! Admin errors.

/// Failure while configuring admin resources.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum AdminError {
    /// Two resources share the same name.
    #[error("admin resource `{name}` is already registered")]
    DuplicateResource {
        /// Resource name.
        name: String,
    },
}
