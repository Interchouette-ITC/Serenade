//! Process-local marking store.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::marking::Marking;
use crate::store::MarkingStore;

/// In-memory [`MarkingStore`] for tests and single-process apps.
#[derive(Debug, Default)]
pub struct MemoryMarkingStore {
    inner: Mutex<HashMap<String, Marking>>,
}

impl MemoryMarkingStore {
    /// Empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl MarkingStore for MemoryMarkingStore {
    fn get(&self, subject_id: &str) -> Option<Marking> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(subject_id)
            .cloned()
    }

    fn set(&self, subject_id: &str, marking: Marking) {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(subject_id.to_owned(), marking);
    }

    fn clear(&self, subject_id: &str) {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(subject_id);
    }
}
