//! Serde bridge helpers for typed JSON encode/decode.

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::encoder::{JsonDecoder, JsonEncoder, FORMAT_JSON};
use crate::{Decoder, Encoder, SerializerError};

/// Serializes `value` to bytes for `format` via serde (JSON only in v0).
///
/// # Errors
///
/// Returns [`SerializerError::UnsupportedFormat`] for non-JSON formats, or
/// [`SerializerError::Codec`] when `serde_json` fails.
///
/// # Examples
///
/// ```
/// use serde::Serialize;
/// use serenade_serializer::serialize_value;
///
/// #[derive(Serialize)]
/// struct Sample { code: String }
///
/// let bytes = serialize_value(&Sample { code: "A".into() }, "json").expect("ok");
/// assert!(bytes.starts_with(b"{"));
/// ```
pub fn serialize_value<T: Serialize>(value: &T, format: &str) -> Result<Vec<u8>, SerializerError> {
    let data = serde_json::to_value(value).map_err(|err| crate::error::from_serde_json(&err))?;
    JsonEncoder.encode(&data, format)
}

/// Deserializes `data` into `T` for `format` via serde (JSON only in v0).
///
/// # Errors
///
/// Returns [`SerializerError::UnsupportedFormat`] for non-JSON formats, or
/// [`SerializerError::Codec`] when decoding or serde mapping fails.
pub fn deserialize_value<T: DeserializeOwned>(
    data: &[u8],
    format: &str,
) -> Result<T, SerializerError> {
    let value = JsonDecoder.decode(data, format)?;
    serde_json::from_value(value).map_err(|err| crate::error::from_serde_json(&err))
}

/// Returns whether the serde bridge supports `format` in this crate version.
#[must_use]
pub fn serde_supports_format(format: &str) -> bool {
    format.eq_ignore_ascii_case(FORMAT_JSON)
}
