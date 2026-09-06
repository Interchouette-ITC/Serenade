//! Format codecs: intermediate [`serde_json::Value`] ↔ bytes.

use serde_json::Value;

use crate::SerializerError;

/// JSON format name used by the built-in codecs.
pub const FORMAT_JSON: &str = "json";

/// Encodes an intermediate [`Value`] into bytes for a format.
pub trait Encoder: Send + Sync {
    /// Returns `true` when this encoder handles `format`.
    fn supports(&self, format: &str) -> bool;

    /// Encodes `data` for `format`.
    ///
    /// # Errors
    ///
    /// Returns [`SerializerError::Codec`] when encoding fails, or
    /// [`SerializerError::UnsupportedFormat`] when the format is not supported.
    fn encode(&self, data: &Value, format: &str) -> Result<Vec<u8>, SerializerError>;
}

/// Decodes bytes into an intermediate [`Value`] for a format.
pub trait Decoder: Send + Sync {
    /// Returns `true` when this decoder handles `format`.
    fn supports(&self, format: &str) -> bool;

    /// Decodes `data` for `format`.
    ///
    /// # Errors
    ///
    /// Returns [`SerializerError::Codec`] when decoding fails, or
    /// [`SerializerError::UnsupportedFormat`] when the format is not supported.
    fn decode(&self, data: &[u8], format: &str) -> Result<Value, SerializerError>;
}

/// JSON encoder (`serde_json`).
#[derive(Debug, Default, Clone, Copy)]
pub struct JsonEncoder;

impl Encoder for JsonEncoder {
    fn supports(&self, format: &str) -> bool {
        format.eq_ignore_ascii_case(FORMAT_JSON)
    }

    fn encode(&self, data: &Value, format: &str) -> Result<Vec<u8>, SerializerError> {
        if !self.supports(format) {
            return Err(SerializerError::UnsupportedFormat {
                format: format.to_owned(),
            });
        }
        serde_json::to_vec(data).map_err(|err| SerializerError::Codec {
            message: err.to_string(),
        })
    }
}

/// JSON decoder (`serde_json`).
#[derive(Debug, Default, Clone, Copy)]
pub struct JsonDecoder;

impl Decoder for JsonDecoder {
    fn supports(&self, format: &str) -> bool {
        format.eq_ignore_ascii_case(FORMAT_JSON)
    }

    fn decode(&self, data: &[u8], format: &str) -> Result<Value, SerializerError> {
        if !self.supports(format) {
            return Err(SerializerError::UnsupportedFormat {
                format: format.to_owned(),
            });
        }
        serde_json::from_slice(data).map_err(|err| SerializerError::Codec {
            message: err.to_string(),
        })
    }
}
