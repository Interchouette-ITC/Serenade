//! Session persistence contract.

use std::collections::HashMap;

use crate::SessionError;

/// Loads and saves session attribute maps by id.
pub trait SessionStore: Send + Sync {
    /// Returns stored attributes for `id`, or `None` when unknown.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the backend fails.
    fn load(&self, id: &str) -> Result<Option<HashMap<String, String>>, SessionError>;

    /// Persists `attributes` for `id`.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the backend fails.
    fn save(&self, id: &str, attributes: &HashMap<String, String>) -> Result<(), SessionError>;

    /// Deletes `id` when present.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the backend fails.
    fn delete(&self, id: &str) -> Result<(), SessionError>;
}
