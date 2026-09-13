//! [`LockError`] variants.

/// Failure while creating or operating a lock.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum LockError {
    /// Resource name is empty or otherwise invalid.
    #[error("invalid lock key `{key}`: {message}")]
    InvalidKey {
        /// Resource that failed validation.
        key: String,
        /// Reason text.
        message: String,
    },
    /// Another token already holds the resource.
    #[error("lock conflict on `{key}`")]
    Conflict {
        /// Contended resource.
        key: String,
    },
    /// This token does not currently hold the resource.
    #[error("lock not held for `{key}`")]
    NotHeld {
        /// Resource that was not held.
        key: String,
    },
    /// Token generation failed.
    #[error("lock token generation failed")]
    Generation,
    /// Underlying store failure.
    #[error("lock store error: {message}")]
    Store {
        /// Reason text.
        message: String,
    },
}
