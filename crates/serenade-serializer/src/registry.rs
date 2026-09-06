//! Registry of normalizers and denormalizers.

use std::any::{Any, TypeId};
use std::sync::Arc;

use serde_json::Value;

use crate::{Denormalizer, NormalizationContext, Normalizer, SerializerError};

/// Collects [`Normalizer`] / [`Denormalizer`] implementations and dispatches by support.
#[derive(Clone, Default)]
pub struct NormalizerRegistry {
    normalizers: Vec<Arc<dyn Normalizer>>,
    denormalizers: Vec<Arc<dyn Denormalizer>>,
}

impl NormalizerRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a normalizer (first matching `supports_normalization` wins).
    pub fn add_normalizer(&mut self, normalizer: impl Normalizer + 'static) {
        self.normalizers.push(Arc::new(normalizer));
    }

    /// Registers a denormalizer (first matching `supports_denormalization` wins).
    pub fn add_denormalizer(&mut self, denormalizer: impl Denormalizer + 'static) {
        self.denormalizers.push(Arc::new(denormalizer));
    }

    /// Number of registered normalizers.
    #[must_use]
    pub fn normalizer_count(&self) -> usize {
        self.normalizers.len()
    }

    /// Number of registered denormalizers.
    #[must_use]
    pub fn denormalizer_count(&self) -> usize {
        self.denormalizers.len()
    }

    /// Normalizes `object` using the first supporting normalizer.
    ///
    /// # Errors
    ///
    /// Returns [`SerializerError::UnsupportedType`] when no normalizer matches.
    pub fn normalize(
        &self,
        object: &dyn Any,
        format: &str,
        context: &NormalizationContext,
    ) -> Result<Value, SerializerError> {
        for normalizer in &self.normalizers {
            if normalizer.supports_normalization(object, format) {
                return normalizer.normalize(object, format, context);
            }
        }
        Err(SerializerError::UnsupportedType {
            format: format.to_owned(),
            message: "no normalizer registered for this type".to_owned(),
        })
    }

    /// Denormalizes `data` into `type_id` using the first supporting denormalizer.
    ///
    /// # Errors
    ///
    /// Returns [`SerializerError::UnsupportedType`] when no denormalizer matches.
    pub fn denormalize(
        &self,
        data: &Value,
        type_id: TypeId,
        format: &str,
        context: &NormalizationContext,
    ) -> Result<Box<dyn Any + Send + Sync>, SerializerError> {
        for denormalizer in &self.denormalizers {
            if denormalizer.supports_denormalization(type_id, format) {
                return denormalizer.denormalize(data, type_id, format, context);
            }
        }
        Err(SerializerError::UnsupportedType {
            format: format.to_owned(),
            message: "no denormalizer registered for this type".to_owned(),
        })
    }
}
