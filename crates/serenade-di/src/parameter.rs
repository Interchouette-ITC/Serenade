//! Parameter bag for container configuration values.

use std::collections::HashMap;

use crate::DiError;

/// String-keyed configuration parameters available during compile and resolve.
#[derive(Clone, Debug, Default)]
pub struct ParameterBag {
    values: HashMap<String, String>,
}

impl ParameterBag {
    /// Creates an empty bag.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts or replaces a parameter.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.values.insert(key.into(), value.into());
    }

    /// Returns a parameter value.
    ///
    /// # Errors
    ///
    /// Returns [`DiError::ParameterNotFound`] when the key is absent.
    pub fn get(&self, key: &str) -> Result<&str, DiError> {
        self.values
            .get(key)
            .map(String::as_str)
            .ok_or_else(|| DiError::ParameterNotFound(key.to_owned()))
    }

    /// Returns whether the key exists.
    #[must_use]
    pub fn contains(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }
}
