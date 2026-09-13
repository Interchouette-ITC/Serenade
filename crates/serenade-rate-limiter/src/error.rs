//! [`RateLimiterError`] variants.

/// Failure while creating or consuming a rate limiter.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum RateLimiterError {
    /// Limiter or subject key is empty or otherwise invalid.
    #[error("invalid rate-limiter key `{key}`: {message}")]
    InvalidKey {
        /// Key that failed validation.
        key: String,
        /// Reason text.
        message: String,
    },
    /// Token count was zero when a positive consume was required, or overflowed.
    #[error("invalid token count `{tokens}`: {message}")]
    InvalidTokens {
        /// Requested token count.
        tokens: u32,
        /// Reason text.
        message: String,
    },
    /// Underlying storage failure.
    #[error("rate-limiter storage error: {message}")]
    Storage {
        /// Reason text.
        message: String,
    },
}
