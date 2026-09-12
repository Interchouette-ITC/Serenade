//! Indexed document shape.

use std::collections::BTreeMap;

/// Document stored in a [`DocumentIndex`](crate::DocumentIndex).
///
/// `id` is the stable application key. `fields` holds searchable text keyed by
/// field name (for example `title`, `body`). Field names are opaque to the
/// adapter; matching concatenates values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchDocument {
    id: String,
    fields: BTreeMap<String, String>,
}

impl SearchDocument {
    /// Builds a document with `id` and no fields.
    ///
    /// # Examples
    ///
    /// ```
    /// use serenade_search::SearchDocument;
    ///
    /// let doc = SearchDocument::new("post-1").field("body", "hello feed");
    /// assert_eq!(doc.id(), "post-1");
    /// assert_eq!(doc.fields().get("body").map(String::as_str), Some("hello feed"));
    /// ```
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            fields: BTreeMap::new(),
        }
    }

    /// Adds or replaces a text field.
    #[must_use]
    pub fn field(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.insert(name.into(), value.into());
        self
    }

    /// Stable document id.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Searchable text fields.
    #[must_use]
    pub const fn fields(&self) -> &BTreeMap<String, String> {
        &self.fields
    }

    /// Consumes the document into `(id, fields)`.
    #[must_use]
    pub fn into_parts(self) -> (String, BTreeMap<String, String>) {
        (self.id, self.fields)
    }
}
