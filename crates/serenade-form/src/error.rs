//! Form component errors.

use serenade_security::SecurityError;

/// Form bind / CSRF / parse failure.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum FormError {
    /// Request body is not valid `application/x-www-form-urlencoded`.
    #[error("invalid form body encoding")]
    InvalidEncoding,
    /// CSRF check failed.
    #[error(transparent)]
    Csrf(#[from] SecurityError),
    /// Form was not submitted (wrong method or empty bind).
    #[error("form not submitted")]
    NotSubmitted,
}
