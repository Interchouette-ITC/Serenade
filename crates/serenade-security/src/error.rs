//! Security component errors.

/// AuthN/Z failure.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum SecurityError {
    /// Authenticator rejected the credentials.
    #[error("authentication failed: {message}")]
    Authentication {
        /// Reason text.
        message: String,
    },
    /// Access decision denied the subject.
    #[error("access denied: {message}")]
    AccessDenied {
        /// Reason text.
        message: String,
    },
}
