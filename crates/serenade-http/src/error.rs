//! HTTP handler and kernel errors.

/// Failure while handling a request.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum HttpError {
    /// Handler failed without a specific status (maps to HTTP 500).
    #[error("request handler failed: {0}")]
    Failed(String),
    /// Handler failed with an HTTP status to map onto the response.
    #[error("HTTP {status}: {message}")]
    Status {
        /// Suggested status code.
        status: u16,
        /// Error text.
        message: String,
    },
}

impl HttpError {
    /// Handler failure that maps to HTTP 500 by default.
    #[must_use]
    pub fn failed(message: impl Into<String>) -> Self {
        Self::Failed(message.into())
    }

    /// Failure that maps to `status`.
    #[must_use]
    pub fn status(status: u16, message: impl Into<String>) -> Self {
        Self::Status {
            status,
            message: message.into(),
        }
    }

    /// Convenience: HTTP 404.
    #[must_use]
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::status(404, message)
    }

    /// Convenience: HTTP 400.
    #[must_use]
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::status(400, message)
    }

    /// Convenience: HTTP 422.
    #[must_use]
    pub fn unprocessable(message: impl Into<String>) -> Self {
        Self::status(422, message)
    }

    /// Status used by [`crate::DefaultExceptionHandler`].
    #[must_use]
    pub const fn status_code(&self) -> u16 {
        match self {
            Self::Failed(_) => 500,
            Self::Status { status, .. } => *status,
        }
    }

    /// Human-readable message (without the `HTTP {status}:` prefix).
    #[must_use]
    pub fn message(&self) -> &str {
        match self {
            Self::Failed(message) | Self::Status { message, .. } => message,
        }
    }
}
