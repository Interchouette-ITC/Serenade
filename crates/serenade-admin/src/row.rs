//! One list/show row (string map; apps map entities here).

use std::collections::BTreeMap;

/// Display row for admin HTML.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AdminRow {
    id: String,
    values: BTreeMap<String, String>,
}

impl AdminRow {
    /// Creates a row with id and optional field map.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            values: BTreeMap::new(),
        }
    }

    /// Sets a field value.
    #[must_use]
    pub fn with(mut self, field: impl Into<String>, value: impl Into<String>) -> Self {
        self.values.insert(field.into(), value.into());
        self
    }

    /// Row id (path `{id}`).
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Field value or empty string.
    #[must_use]
    pub fn get(&self, field: &str) -> &str {
        self.values.get(field).map_or("", String::as_str)
    }
}
