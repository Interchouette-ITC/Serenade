//! Security component errors.

/// AuthN/Z and CSRF failure.
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
    /// CSRF token missing, malformed, or invalid.
    #[error("invalid CSRF token")]
    InvalidCsrfToken,
    /// CSRF token generation failed (RNG).
    #[error("CSRF token generation failed")]
    CsrfGeneration,
    /// Password hash or verify failed.
    #[error("password error: {message}")]
    Password {
        /// Underlying message.
        message: String,
    },
    /// OAuth / OIDC client helper failed.
    #[error("oauth error: {message}")]
    OAuth {
        /// Underlying message.
        message: String,
    },
}
