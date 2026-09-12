//! In-request session bag (Symfony `SessionInterface` / attribute bag analogue).

use std::collections::HashMap;

/// Mutable key/value session attributes for one request lifecycle.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Session {
    id: String,
    attributes: HashMap<String, String>,
    is_new: bool,
    invalidated: bool,
    dirty: bool,
}

impl Session {
    /// Creates a new empty session with `id`.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            attributes: HashMap::new(),
            is_new: true,
            invalidated: false,
            dirty: false,
        }
    }

    /// Creates a session loaded from a store.
    #[must_use]
    pub fn existing(id: impl Into<String>, attributes: HashMap<String, String>) -> Self {
        Self {
            id: id.into(),
            attributes,
            is_new: false,
            invalidated: false,
            dirty: false,
        }
    }

    /// Session identifier (opaque; placed in the session cookie).
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Whether this id was minted for the current request.
    #[must_use]
    pub const fn is_new(&self) -> bool {
        self.is_new
    }

    /// Whether [`Self::invalidate`] was called.
    #[must_use]
    pub const fn is_invalidated(&self) -> bool {
        self.invalidated
    }

    /// Whether attributes changed since load (or the session is new).
    #[must_use]
    pub const fn is_dirty(&self) -> bool {
        self.dirty || self.is_new
    }

    /// Borrowed attribute value.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.attributes.get(key).map(String::as_str)
    }

    /// Sets an attribute.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.attributes.insert(key.into(), value.into());
        self.dirty = true;
    }

    /// Removes an attribute.
    pub fn remove(&mut self, key: &str) -> Option<String> {
        let previous = self.attributes.remove(key);
        if previous.is_some() {
            self.dirty = true;
        }
        previous
    }

    /// Clears all attributes (keeps the same id unless invalidated).
    pub fn clear(&mut self) {
        if !self.attributes.is_empty() {
            self.attributes.clear();
            self.dirty = true;
        }
    }

    /// Marks the session destroyed: attributes cleared, store delete on commit.
    pub fn invalidate(&mut self) {
        self.attributes.clear();
        self.invalidated = true;
        self.dirty = true;
    }

    /// Attribute map for persistence.
    #[must_use]
    pub const fn attributes(&self) -> &HashMap<String, String> {
        &self.attributes
    }

    pub(crate) fn take_for_save(&self) -> HashMap<String, String> {
        self.attributes.clone()
    }
}
