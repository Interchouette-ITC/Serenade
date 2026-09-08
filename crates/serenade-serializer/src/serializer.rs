//! Facade that normalizes then encodes, or decodes then denormalizes.

use std::any::{Any, TypeId};
use std::sync::Arc;

use crate::encoder::{JsonDecoder, JsonEncoder};
#[cfg(feature = "toon")]
use crate::encoder::{ToonDecoder, ToonEncoder};
use crate::{Decoder, Encoder, NormalizationContext, NormalizerRegistry, SerializerError};

/// Composes a [`NormalizerRegistry`] with format codecs (`SerializerInterface` analogue).
///
/// # Examples
///
/// ```
/// use std::any::{Any, TypeId};
///
/// use serde_json::{json, Value};
/// use serenade_serializer::{
///     Denormalizer, FORMAT_JSON, NormalizationContext, Normalizer, Serializer,
///     SerializerError,
/// };
///
/// struct Tag(String);
///
/// struct TagNormalizer;
///
/// impl Normalizer for TagNormalizer {
///     fn supports_normalization(&self, object: &dyn Any, format: &str) -> bool {
///         format == FORMAT_JSON && object.is::<Tag>()
///     }
///
///     fn normalize(
///         &self,
///         object: &dyn Any,
///         _format: &str,
///         _context: &NormalizationContext,
///     ) -> Result<Value, SerializerError> {
///         let tag = object.downcast_ref::<Tag>().expect("checked");
///         Ok(json!({ "label": tag.0 }))
///     }
/// }
///
/// impl Denormalizer for TagNormalizer {
///     fn supports_denormalization(&self, type_id: TypeId, format: &str) -> bool {
///         format == FORMAT_JSON && type_id == TypeId::of::<Tag>()
///     }
///
///     fn denormalize(
///         &self,
///         data: &Value,
///         _type_id: TypeId,
///         _format: &str,
///         _context: &NormalizationContext,
///     ) -> Result<Box<dyn Any + Send + Sync>, SerializerError> {
///         let label = data
///             .get("label")
///             .and_then(Value::as_str)
///             .ok_or_else(|| SerializerError::Normalization {
///                 message: "missing label".into(),
///             })?;
///         Ok(Box::new(Tag(label.to_owned())))
///     }
/// }
///
/// let mut serializer = Serializer::new();
/// serializer.registry_mut().add_normalizer(TagNormalizer);
/// serializer.registry_mut().add_denormalizer(TagNormalizer);
/// let bytes = serializer.serialize(&Tag("x".into()), FORMAT_JSON).expect("ok");
/// let boxed = serializer
///     .deserialize(&bytes, TypeId::of::<Tag>(), FORMAT_JSON)
///     .expect("ok");
/// assert_eq!(boxed.downcast_ref::<Tag>().unwrap().0, "x");
/// ```
#[derive(Clone)]
pub struct Serializer {
    registry: NormalizerRegistry,
    encoders: Vec<Arc<dyn Encoder>>,
    decoders: Vec<Arc<dyn Decoder>>,
}

impl Default for Serializer {
    fn default() -> Self {
        Self::new()
    }
}

impl Serializer {
    /// Creates a serializer with JSON codecs (and TOON when feature `toon` is on).
    #[must_use]
    pub fn new() -> Self {
        #[cfg(feature = "toon")]
        {
            Self {
                registry: NormalizerRegistry::new(),
                encoders: vec![Arc::new(JsonEncoder), Arc::new(ToonEncoder)],
                decoders: vec![Arc::new(JsonDecoder), Arc::new(ToonDecoder)],
            }
        }
        #[cfg(not(feature = "toon"))]
        {
            Self {
                registry: NormalizerRegistry::new(),
                encoders: vec![Arc::new(JsonEncoder)],
                decoders: vec![Arc::new(JsonDecoder)],
            }
        }
    }

    /// Mutable access to the normalizer registry.
    pub const fn registry_mut(&mut self) -> &mut NormalizerRegistry {
        &mut self.registry
    }

    /// Shared access to the normalizer registry.
    #[must_use]
    pub const fn registry(&self) -> &NormalizerRegistry {
        &self.registry
    }

    /// Appends an encoder (first matching `supports` wins).
    pub fn add_encoder(&mut self, encoder: impl Encoder + 'static) {
        self.encoders.push(Arc::new(encoder));
    }

    /// Appends a decoder (first matching `supports` wins).
    pub fn add_decoder(&mut self, decoder: impl Decoder + 'static) {
        self.decoders.push(Arc::new(decoder));
    }

    /// Normalizes `object` then encodes it for `format`.
    ///
    /// # Errors
    ///
    /// Returns registry or codec [`SerializerError`] variants.
    pub fn serialize(&self, object: &dyn Any, format: &str) -> Result<Vec<u8>, SerializerError> {
        self.serialize_with_context(object, format, &NormalizationContext::new())
    }

    /// Like [`Self::serialize`] with an explicit context.
    ///
    /// # Errors
    ///
    /// Returns registry or codec [`SerializerError`] variants.
    pub fn serialize_with_context(
        &self,
        object: &dyn Any,
        format: &str,
        context: &NormalizationContext,
    ) -> Result<Vec<u8>, SerializerError> {
        let data = self.registry.normalize(object, format, context)?;
        self.encode(&data, format)
    }

    /// Decodes `data` then denormalizes into `type_id`.
    ///
    /// # Errors
    ///
    /// Returns codec or registry [`SerializerError`] variants.
    pub fn deserialize(
        &self,
        data: &[u8],
        type_id: TypeId,
        format: &str,
    ) -> Result<Box<dyn Any + Send + Sync>, SerializerError> {
        self.deserialize_with_context(data, type_id, format, &NormalizationContext::new())
    }

    /// Like [`Self::deserialize`] with an explicit context.
    ///
    /// # Errors
    ///
    /// Returns codec or registry [`SerializerError`] variants.
    pub fn deserialize_with_context(
        &self,
        data: &[u8],
        type_id: TypeId,
        format: &str,
        context: &NormalizationContext,
    ) -> Result<Box<dyn Any + Send + Sync>, SerializerError> {
        let value = self.decode(data, format)?;
        self.registry.denormalize(&value, type_id, format, context)
    }

    fn encode(&self, data: &serde_json::Value, format: &str) -> Result<Vec<u8>, SerializerError> {
        for encoder in &self.encoders {
            if encoder.supports(format) {
                return encoder.encode(data, format);
            }
        }
        Err(SerializerError::UnsupportedFormat {
            format: format.to_owned(),
        })
    }

    fn decode(&self, data: &[u8], format: &str) -> Result<serde_json::Value, SerializerError> {
        for decoder in &self.decoders {
            if decoder.supports(format) {
                return decoder.decode(data, format);
            }
        }
        Err(SerializerError::UnsupportedFormat {
            format: format.to_owned(),
        })
    }
}
