//! Serializer errors.

/// Failure while normalizing, denormalizing, encoding, or decoding.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum SerializerError {
    /// No encoder or decoder supports the requested format.
    #[error("unsupported format `{format}`")]
    UnsupportedFormat {
        /// Format name (for example `json`).
        format: String,
    },
    /// No normalizer or denormalizer accepts the value or type.
    #[error("unsupported type for format `{format}`: {message}")]
    UnsupportedType {
        /// Format name being used.
        format: String,
        /// Detail about the missing handler.
        message: String,
    },
    /// Encoding or decoding the wire format failed.
    #[error("codec error: {message}")]
    Codec {
        /// Underlying codec message.
        message: String,
    },
    /// Normalization or denormalization failed.
    #[error("normalization error: {message}")]
    Normalization {
        /// Underlying normalization message.
        message: String,
    },
}

/// Maps a `serde_json` error into [`SerializerError::Codec`].
#[must_use]
pub fn from_serde_json(err: serde_json::Error) -> SerializerError {
    SerializerError::Codec {
        message: err.to_string(),
    }
}
