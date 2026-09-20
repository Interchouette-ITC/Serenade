//! Persist markings for subjects.

use crate::marking::Marking;

/// Load and save a subject's marking.
pub trait MarkingStore: Send + Sync {
    /// Current marking, if any has been stored.
    fn get(&self, subject_id: &str) -> Option<Marking>;

    /// Replace the stored marking for `subject_id`.
    fn set(&self, subject_id: &str, marking: Marking);

    /// Remove a stored marking (subject falls back to definition initial).
    fn clear(&self, subject_id: &str);
}
