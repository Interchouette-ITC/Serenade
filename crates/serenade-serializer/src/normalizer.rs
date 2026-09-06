//! Object graph normalizers and denormalizers.

use std::any::{Any, TypeId};

use serde_json::Value;

use crate::{NormalizationContext, SerializerError};

/// Turns a typed object into an intermediate [`Value`].
pub trait Normalizer: Send + Sync {
    /// Returns `true` when this normalizer can handle `object` for `format`.
    fn supports_normalization(&self, object: &dyn Any, format: &str) -> bool;

    /// Normalizes `object` for `format`.
    ///
    /// # Errors
    ///
    /// Returns [`SerializerError`] when normalization fails or the type is unsupported.
    fn normalize(
        &self,
        object: &dyn Any,
        format: &str,
        context: &NormalizationContext,
    ) -> Result<Value, SerializerError>;
}

/// Turns an intermediate [`Value`] into a boxed object of a target [`TypeId`].
pub trait Denormalizer: Send + Sync {
    /// Returns `true` when this denormalizer can produce `type_id` for `format`.
    fn supports_denormalization(&self, type_id: TypeId, format: &str) -> bool;

    /// Denormalizes `data` into a boxed value of `type_id`.
    ///
    /// # Errors
    ///
    /// Returns [`SerializerError`] when denormalization fails or the type is unsupported.
    fn denormalize(
        &self,
        data: &Value,
        type_id: TypeId,
        format: &str,
        context: &NormalizationContext,
    ) -> Result<Box<dyn Any + Send + Sync>, SerializerError>;
}
