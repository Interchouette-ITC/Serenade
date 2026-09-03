//! Case-insensitive HTTP header map (last value wins).

use std::collections::HashMap;

/// Header names stored in lowercase; lookup is case-insensitive.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Headers {
    values: HashMap<String, String>,
}

impl Headers {
    /// Empty header map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts or replaces a header. The name is stored in lowercase.
    pub fn insert(&mut self, name: impl AsRef<str>, value: impl Into<String>) {
        self.values
            .insert(name.as_ref().to_ascii_lowercase(), value.into());
    }

    /// Returns the value for `name`, if present.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.values
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }

    /// Number of stored headers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Whether the map is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Iterates `(lowercase name, value)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.values.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }
}
