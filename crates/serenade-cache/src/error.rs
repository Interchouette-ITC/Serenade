//! Cache errors.

/// Failure while reading or writing cache items.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum CacheError {
    /// Key is invalid for this pool.
    #[error("invalid cache key `{key}`: {message}")]
    InvalidKey {
        /// Cache key.
        key: String,
        /// Reason.
        message: String,
    },
    /// Pool operation failed.
    #[error("cache pool error: {message}")]
    Pool {
        /// Underlying message.
        message: String,
    },
}
