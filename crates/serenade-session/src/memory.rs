//! In-memory [`SessionStore`](crate::SessionStore).

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use crate::{SessionError, SessionStore};

/// Process-local session store (tests and single-node dev).
#[derive(Default)]
pub struct MemorySessionStore {
    inner: Mutex<HashMap<String, HashMap<String, String>>>,
}

impl MemorySessionStore {
    /// Creates an empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<String, HashMap<String, String>>> {
        self.inner
            .lock()
            .expect("serenade-session MemorySessionStore mutex poisoned")
    }
}

impl SessionStore for MemorySessionStore {
    fn load(&self, id: &str) -> Result<Option<HashMap<String, String>>, SessionError> {
        validate_id(id)?;
        Ok(self.lock().get(id).cloned())
    }

    fn save(&self, id: &str, attributes: &HashMap<String, String>) -> Result<(), SessionError> {
        validate_id(id)?;
        self.lock().insert(id.to_owned(), attributes.clone());
        Ok(())
    }

    fn delete(&self, id: &str) -> Result<(), SessionError> {
        validate_id(id)?;
        self.lock().remove(id);
        Ok(())
    }
}

fn validate_id(id: &str) -> Result<(), SessionError> {
    if id.is_empty() {
        return Err(SessionError::InvalidId {
            message: "id must not be empty".to_owned(),
        });
    }
    Ok(())
}
