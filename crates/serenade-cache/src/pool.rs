//! Cache item pool trait and in-memory adapter.

use std::any::Any;
use std::collections::{HashMap, HashSet};
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

    /// Deletes every item tagged with any of `tags`. Returns how many keys were removed.
    ///
    /// Default: not supported (returns [`CacheError::Pool`]). [`ArrayAdapter`] implements tags.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] when the pool does not support tags or delete fails.
    fn invalidate_tags(&self, tags: &[&str]) -> Result<usize, CacheError> {
        let _ = tags;
        Err(CacheError::Pool {
            message: "tag invalidation is not supported by this pool".to_owned(),
        })
    }
}

struct Stored {
    value: Arc<dyn Any + Send + Sync>,
    expires_at: Option<Instant>,
    tags: Vec<String>,
}

#[derive(Default)]
struct Inner {
    items: HashMap<String, Stored>,
    /// Tag → keys currently associated with that tag.
    by_tag: HashMap<String, HashSet<String>>,
}

/// In-memory [`CacheItemPool`] (`ArrayAdapter`).
#[derive(Default)]
pub struct ArrayAdapter {
    inner: Mutex<Inner>,
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

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .expect("serenade-cache ArrayAdapter mutex poisoned")
    }
}

fn unlink_key_tags(inner: &mut Inner, key: &str, tags: &[String]) {
    for tag in tags {
        if let Some(keys) = inner.by_tag.get_mut(tag) {
            keys.remove(key);
            if keys.is_empty() {
                inner.by_tag.remove(tag);
            }
        }
    }
}

fn link_key_tags(inner: &mut Inner, key: &str, tags: &[String]) {
    for tag in tags {
        inner
            .by_tag
            .entry(tag.clone())
            .or_default()
            .insert(key.to_owned());
    }
}

impl CacheItemPool for ArrayAdapter {
    fn get_item(&self, key: &str) -> Result<ArrayCacheItem, CacheError> {
        Self::validate_key(key)?;
        let mut inner = self.lock();
        if inner
            .items
            .get(key)
            .is_some_and(|stored| stored.expires_at.is_some_and(|at| Instant::now() >= at))
        {
            let stored = inner
                .items
                .remove(key)
                .expect("expired key was present before remove");
            unlink_key_tags(&mut inner, key, &stored.tags);
            return Ok(ArrayCacheItem::miss(key));
        }
        let item = inner.items.get(key).map_or_else(
            || ArrayCacheItem::miss(key),
            |stored| {
                ArrayCacheItem::hit(key, Arc::clone(&stored.value))
                    .with_expiry(stored.expires_at)
                    .with_tags(stored.tags.clone())
            },
        );
        drop(inner);
        Ok(item)
    }

    fn save(&self, item: ArrayCacheItem) -> Result<(), CacheError> {
        let (key, value, expires_at, tags) = item.into_stored();
        Self::validate_key(&key)?;
        let mut inner = self.lock();
        if let Some(previous) = inner.items.remove(&key) {
            unlink_key_tags(&mut inner, &key, &previous.tags);
        }
        let Some(value) = value else {
            drop(inner);
            return Ok(());
        };
        if expires_at.is_some_and(|at| Instant::now() >= at) {
            drop(inner);
            return Ok(());
        }
        link_key_tags(&mut inner, &key, &tags);
        inner.items.insert(
            key,
            Stored {
                value,
                expires_at,
                tags,
            },
        );
        drop(inner);
        Ok(())
    }

    fn delete_item(&self, key: &str) -> Result<bool, CacheError> {
        Self::validate_key(key)?;
        let mut inner = self.lock();
        let Some(stored) = inner.items.remove(key) else {
            drop(inner);
            return Ok(false);
        };
        unlink_key_tags(&mut inner, key, &stored.tags);
        drop(inner);
        Ok(true)
    }

    fn clear(&self) -> Result<(), CacheError> {
        let mut inner = self.lock();
        inner.items.clear();
        inner.by_tag.clear();
        drop(inner);
        Ok(())
    }

    fn invalidate_tags(&self, tags: &[&str]) -> Result<usize, CacheError> {
        let mut inner = self.lock();
        let mut keys_to_remove = HashSet::new();
        for tag in tags {
            if tag.is_empty() {
                continue;
            }
            if let Some(keys) = inner.by_tag.remove(*tag) {
                keys_to_remove.extend(keys);
            }
        }
        let mut removed = 0;
        let keys: Vec<String> = keys_to_remove
            .into_iter()
            .filter(|key| inner.items.contains_key(key))
            .collect();
        for key in keys {
            let stored = inner
                .items
                .remove(&key)
                .expect("key was present before remove");
            unlink_key_tags(&mut inner, &key, &stored.tags);
            removed += 1;
        }
        drop(inner);
        Ok(removed)
    }
}
