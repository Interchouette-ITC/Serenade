//! Search adapter errors.

/// Failure while indexing or querying documents.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum SearchError {
    /// Document id is invalid for this index.
    #[error("invalid search document id `{id}`: {message}")]
    InvalidId {
        /// Document id.
        id: String,
        /// Reason.
        message: String,
    },
    /// Query text is invalid for this index.
    #[error("invalid search query: {message}")]
    InvalidQuery {
        /// Reason.
        message: String,
    },
    /// Index operation failed.
    #[error("search index error: {message}")]
    Index {
        /// Underlying message.
        message: String,
    },
}
