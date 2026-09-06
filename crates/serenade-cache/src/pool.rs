//! Cache item pool trait and in-memory adapter.

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

use crate::{ArrayCacheItem, CacheError, CacheItem};

/// Pool that loads and persists [`CacheItem`](crate::CacheItem)s (PSR-6 analogue).
pub trait CacheItemPool: Send + Sync {
    /// Returns an item for `key` (miss when absent or expired).
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] when the key is invalid or the pool fails.
    fn get_item(&self, key: &str) -> Result<ArrayCacheItem, CacheError>;

    /// Persists `item`.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] when save fails.
    fn save(&self, item: ArrayCacheItem) -> Result<(), CacheError>;

    /// Deletes `key` when present.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] when delete fails.
    fn delete_item(&self, key: &str) -> Result<bool, CacheError>;

    /// Clears all items.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] when clear fails.
    fn clear(&self) -> Result<(), CacheError>;

    /// Returns whether `key` is a cache hit.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] when the lookup fails.
    fn has_item(&self, key: &str) -> Result<bool, CacheError> {
        Ok(self.get_item(key)?.is_hit())
    }

    /// Loads many keys (default: sequential [`Self::get_item`]).
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] when any lookup fails.
    fn get_items(&self, keys: &[&str]) -> Result<Vec<ArrayCacheItem>, CacheError> {
        keys.iter().map(|key| self.get_item(key)).collect()
    }

    /// Deletes many keys (default: sequential [`Self::delete_item`]).
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] when any delete fails.
    fn delete_items(&self, keys: &[&str]) -> Result<usize, CacheError> {
        let mut removed = 0;
        for key in keys {
            if self.delete_item(key)? {
                removed += 1;
            }
        }
        Ok(removed)
    }
}

struct Stored {
    value: Option<Arc<dyn Any + Send + Sync>>,
    expires_at: Option<Instant>,
}

/// In-memory [`CacheItemPool`] (`ArrayAdapter`).
#[derive(Default)]
pub struct ArrayAdapter {
    inner: Mutex<HashMap<String, Stored>>,
}

impl ArrayAdapter {
    /// Creates an empty in-memory pool.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn validate_key(key: &str) -> Result<(), CacheError> {
        if key.is_empty() {
            return Err(CacheError::InvalidKey {
                key: key.to_owned(),
                message: "key must not be empty".to_owned(),
            });
        }
        Ok(())
    }

    fn lock_map(&self) -> MutexGuard<'_, HashMap<String, Stored>> {
        self.inner
            .lock()
            .expect("serenade-cache ArrayAdapter mutex poisoned")
    }
}

impl CacheItemPool for ArrayAdapter {
    fn get_item(&self, key: &str) -> Result<ArrayCacheItem, CacheError> {
        Self::validate_key(key)?;
        let mut map = self.lock_map();
        if map
            .get(key)
            .is_some_and(|stored| stored.expires_at.is_some_and(|at| Instant::now() >= at))
        {
            map.remove(key);
            return Ok(ArrayCacheItem::miss(key));
        }
        let item = map.get(key).map_or_else(
            || ArrayCacheItem::miss(key),
            |stored| {
                stored.value.as_ref().map_or_else(
                    || ArrayCacheItem::miss(key),
                    |value| {
                        ArrayCacheItem::hit(key, Arc::clone(value)).with_expiry(stored.expires_at)
                    },
                )
            },
        );
        drop(map);
        Ok(item)
    }

    fn save(&self, item: ArrayCacheItem) -> Result<(), CacheError> {
        let (key, value, expires_at) = item.into_stored();
        Self::validate_key(&key)?;
        self.lock_map().insert(key, Stored { value, expires_at });
        Ok(())
    }

    fn delete_item(&self, key: &str) -> Result<bool, CacheError> {
        Self::validate_key(key)?;
        Ok(self.lock_map().remove(key).is_some())
    }

    fn clear(&self) -> Result<(), CacheError> {
        self.lock_map().clear();
        Ok(())
    }
}
