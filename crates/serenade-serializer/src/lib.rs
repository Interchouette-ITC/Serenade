//! DTO serialization: JSON codecs, optional TOON, normalizer registry, serde bridge.
//!
//! Typed serde helpers cover the common JSON path. Custom types register
//! [`Normalizer`] / [`Denormalizer`] implementations on a [`NormalizerRegistry`]
//! (or via [`Serializer`]). Enable Cargo feature `toon` for TOON v4.1 LLM export.

mod context;
mod encoder;
mod error;
mod normalizer;
mod registry;
mod serde_bridge;
mod serializer;

pub use context::NormalizationContext;
pub use encoder::{Decoder, Encoder, FORMAT_JSON, JsonDecoder, JsonEncoder};
#[cfg(feature = "toon")]
pub use encoder::{FORMAT_TOON, ToonDecoder, ToonEncoder, encode_toon_string};
pub use error::{SerializerError, from_serde_json};
pub use normalizer::{Denormalizer, Normalizer};
pub use registry::NormalizerRegistry;
pub use serde_bridge::{deserialize_value, serde_supports_format, serialize_value};
pub use serializer::Serializer;

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
