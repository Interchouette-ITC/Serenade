//! Process-local [`LockStore`].

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::{LockError, LockStore, validate_resource};

#[derive(Debug)]
struct Entry {
    token: String,
    expires_at: Option<Instant>,
}

impl Entry {
    fn is_expired(&self, now: Instant) -> bool {
        self.expires_at.is_some_and(|at| at <= now)
    }
}

/// In-process lock store backed by a [`Mutex`] + [`HashMap`].
#[derive(Debug, Default)]
pub struct InMemoryLockStore {
    inner: Mutex<HashMap<String, Entry>>,
}

impl InMemoryLockStore {
    /// Empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn with_map<T>(&self, f: impl FnOnce(&mut HashMap<String, Entry>) -> T) -> T {
        let mut guard = self
            .inner
            .lock()
            .expect("serenade-lock InMemoryLockStore mutex poisoned");
        f(&mut guard)
    }
}

impl LockStore for InMemoryLockStore {
    fn save(&self, resource: &str, token: &str, ttl: Option<Duration>) -> Result<(), LockError> {
        validate_resource(resource)?;
        let now = Instant::now();
        self.with_map(|map| {
            if let Some(entry) = map.get(resource) {
                if !entry.is_expired(now) && entry.token != token {
                    return Err(LockError::Conflict {
                        key: resource.to_owned(),
                    });
                }
            }
            map.insert(
                resource.to_owned(),
                Entry {
                    token: token.to_owned(),
                    expires_at: ttl.map(|duration| now + duration),
                },
            );
            Ok(())
        })
    }

    fn delete(&self, resource: &str, token: &str) -> Result<(), LockError> {
        validate_resource(resource)?;
        let now = Instant::now();
        self.with_map(|map| {
            match map.get(resource) {
                Some(entry) if entry.is_expired(now) => {
                    map.remove(resource);
                }
                Some(entry) if entry.token == token => {
                    map.remove(resource);
                }
                Some(_) | None => {}
            }
            Ok(())
        })
    }

    fn exists(&self, resource: &str, token: &str) -> Result<bool, LockError> {
        validate_resource(resource)?;
        let now = Instant::now();
        self.with_map(|map| {
            let Some(entry) = map.get(resource) else {
                return Ok(false);
            };
            if entry.is_expired(now) {
                map.remove(resource);
                return Ok(false);
            }
            Ok(entry.token == token)
        })
    }

    fn put_off_expiration(
        &self,
        resource: &str,
        token: &str,
        ttl: Option<Duration>,
    ) -> Result<(), LockError> {
        validate_resource(resource)?;
        let now = Instant::now();
        self.with_map(|map| {
            let Some(entry) = map.get_mut(resource) else {
                return Err(LockError::NotHeld {
                    key: resource.to_owned(),
                });
            };
            if entry.is_expired(now) {
                map.remove(resource);
                return Err(LockError::NotHeld {
                    key: resource.to_owned(),
                });
            }
            if entry.token != token {
                return Err(LockError::NotHeld {
                    key: resource.to_owned(),
                });
            }
            entry.expires_at = ttl.map(|duration| now + duration);
            Ok(())
        })
    }
}
