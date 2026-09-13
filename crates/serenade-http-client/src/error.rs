//! [`HttpClientError`] variants.

/// Failure while building or sending an outbound request.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum HttpClientError {
    /// URL or request construction failed.
    #[error("http client request: {message}")]
    Request {
        /// Reason text.
        message: String,
    },
    /// Transport / network failure.
    #[error("http client transport: {message}")]
    Transport {
        /// Reason text.
        message: String,
    },
    /// No mock handler matched (tests only).
    #[error("http client mock: no handler for {method} {url}")]
    MockMiss {
        /// HTTP method.
        method: String,
        /// Request URL.
        url: String,
    },
}
