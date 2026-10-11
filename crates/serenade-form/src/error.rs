//! Form component errors.

use serenade_security::SecurityError;

/// Form bind / CSRF / parse failure.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum FormError {
    /// Request body is not valid `application/x-www-form-urlencoded`.
    #[error("invalid form body encoding")]
    InvalidEncoding,
    /// Multipart body is invalid or exceeds configured limits.
    #[error("invalid multipart form body: {0}")]
    Multipart(String),
    /// Uploaded file rejected (size, MIME type, or I/O).
    #[error("uploaded file rejected: {0}")]
    Upload(String),
    /// CSRF check failed.
    #[error(transparent)]
    Csrf(#[from] SecurityError),
    /// Form was not submitted (wrong method or empty bind).
    #[error("form not submitted")]
    NotSubmitted,
    /// View ↔ model transform failed.
    #[error("form transform failed: {0}")]
    Transform(String),
    /// Flat key path conflicts (leaf vs map) while nesting form data.
    #[error("nested form data conflict at {0}")]
    NestedConflict(String),
}
