//! Session errors.

/// Failure while reading or writing session state.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum SessionError {
    /// Session id is empty or otherwise rejected.
    #[error("invalid session id: {message}")]
    InvalidId {
        /// Reason.
        message: String,
    },
    /// Store or cookie operation failed.
    #[error("session error: {message}")]
    Store {
        /// Underlying message.
        message: String,
    },
    /// Cryptographic RNG failed while creating a session id.
    #[error("session id generation failed")]
    Generation,
}
