//! Request attributes bag (route params, request-scoped values).

use std::any::Any;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

/// Heterogeneous map keyed by string. Values must be `Send + Sync`.
#[derive(Default)]
pub struct AttributeBag {
    values: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl AttributeBag {
    /// Empty bag.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores `value` under `key`, replacing any previous entry.
    pub fn insert<T: Any + Send + Sync>(&mut self, key: impl Into<String>, value: T) {
        self.values.insert(key.into(), Box::new(value));
    }

    /// Borrows a value of type `T` stored at `key`.
    #[must_use]
    pub fn get<T: Any + Send + Sync>(&self, key: &str) -> Option<&T> {
        self.values.get(key).and_then(|value| value.downcast_ref())
    }

    /// Removes and returns a value of type `T` stored at `key`.
    #[must_use]
    pub fn remove<T: Any + Send + Sync>(&mut self, key: &str) -> Option<T> {
        let boxed = self.values.remove(key)?;
        boxed.downcast().ok().map(|value| *value)
    }

    /// Whether `key` is present (any type).
    #[must_use]
    pub fn contains(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Whether the bag is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

impl Debug for AttributeBag {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AttributeBag")
            .field("keys", &self.values.keys().collect::<Vec<_>>())
            .finish()
    }
}
