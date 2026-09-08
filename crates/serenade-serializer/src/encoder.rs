//! Format codecs: intermediate [`serde_json::Value`] ↔ bytes.

use serde_json::Value;

use crate::SerializerError;

/// JSON format name used by the built-in codecs.
pub const FORMAT_JSON: &str = "json";

/// TOON v4.1 format name (LLM / agent context export). Requires feature `toon`.
#[cfg(feature = "toon")]
pub const FORMAT_TOON: &str = "toon";

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
        serde_json::to_vec(data).map_err(|err| crate::error::from_serde_json(&err))
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
        serde_json::from_slice(data).map_err(|err| crate::error::from_serde_json(&err))
    }
}

/// Canonical TOON v4.1 encoder (`reddb-io-toon`).
#[cfg(feature = "toon")]
#[derive(Debug, Default, Clone, Copy)]
pub struct ToonEncoder;

#[cfg(feature = "toon")]
impl Encoder for ToonEncoder {
    fn supports(&self, format: &str) -> bool {
        format.eq_ignore_ascii_case(FORMAT_TOON)
    }

    fn encode(&self, data: &Value, format: &str) -> Result<Vec<u8>, SerializerError> {
        if !self.supports(format) {
            return Err(SerializerError::UnsupportedFormat {
                format: format.to_owned(),
            });
        }
        encode_toon_bytes(data)
    }
}

/// TOON v4.1 decoder (`reddb-io-toon`).
#[cfg(feature = "toon")]
#[derive(Debug, Default, Clone, Copy)]
pub struct ToonDecoder;

#[cfg(feature = "toon")]
impl Decoder for ToonDecoder {
    fn supports(&self, format: &str) -> bool {
        format.eq_ignore_ascii_case(FORMAT_TOON)
    }

    fn decode(&self, data: &[u8], format: &str) -> Result<Value, SerializerError> {
        if !self.supports(format) {
            return Err(SerializerError::UnsupportedFormat {
                format: format.to_owned(),
            });
        }
        let text = std::str::from_utf8(data).map_err(|err| SerializerError::Codec {
            message: err.to_string(),
        })?;
        let toon_value = reddb_io_toon::decode(text).map_err(|err| SerializerError::Codec {
            message: err.to_string(),
        })?;
        Ok(toon_value.to_json_value())
    }
}

/// Encodes an intermediate [`Value`] as a UTF-8 TOON string for LLM prompts.
///
/// # Errors
///
/// Returns [`SerializerError::Codec`] when TOON encoding fails.
///
/// # Examples
///
/// ```
/// use serde_json::json;
/// use serenade_serializer::encode_toon_string;
///
/// let text = encode_toon_string(&json!({ "sku": "HOODIE", "qty": 2 })).expect("ok");
/// assert!(text.contains("sku"));
/// ```
#[cfg(feature = "toon")]
pub fn encode_toon_string(data: &Value) -> Result<String, SerializerError> {
    encode_toon_string_with_options(data, reddb_io_toon::EncodeOptions::default())
}

#[cfg(feature = "toon")]
fn encode_toon_string_with_options(
    data: &Value,
    options: reddb_io_toon::EncodeOptions,
) -> Result<String, SerializerError> {
    let toon_value = reddb_io_toon::Value::from_json_value(data.clone());
    reddb_io_toon::encode_with_options(&toon_value, options).map_err(|err| SerializerError::Codec {
        message: err.to_string(),
    })
}

#[cfg(feature = "toon")]
fn encode_toon_bytes(data: &Value) -> Result<Vec<u8>, SerializerError> {
    Ok(encode_toon_string(data)?.into_bytes())
}

#[cfg(all(test, feature = "toon"))]
mod encode_error_tests {
    use super::{Encoder, FORMAT_TOON, ToonEncoder, encode_toon_string_with_options};
    use crate::SerializerError;
    use serde_json::json;

    #[test]
    fn encode_rejects_shallow_max_depth() {
        let options = reddb_io_toon::EncodeOptions {
            max_depth: 1,
            ..Default::default()
        };
        let err = encode_toon_string_with_options(&json!({ "a": { "b": { "c": 1 } } }), options)
            .expect_err("too deep");
        assert!(matches!(err, SerializerError::Codec { .. }));
    }

    #[test]
    fn toon_encoder_roundtrip_still_works() {
        let bytes = ToonEncoder
            .encode(&json!({ "ok": true }), FORMAT_TOON)
            .expect("encode");
        assert!(!bytes.is_empty());
    }
}
