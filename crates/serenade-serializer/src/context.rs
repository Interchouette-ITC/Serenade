//! Normalization context passed to normalizers and denormalizers.

use std::collections::HashMap;

/// Key/value bag available during normalize / denormalize.
#[derive(Debug, Clone, Default)]
pub struct NormalizationContext {
    attributes: HashMap<String, String>,
}

impl NormalizationContext {
    /// Creates an empty context.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a string attribute.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.attributes.insert(key.into(), value.into());
    }

    /// Returns an attribute when present.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.attributes.get(key).map(String::as_str)
    }
}
