//! Cache item contract and in-memory item.

use std::any::Any;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// One cache entry (PSR-6 `CacheItemInterface` analogue).
pub trait CacheItem: Send + Sync {
    /// Cache key.
    fn key(&self) -> &str;

    /// Whether this item was a hit when loaded from the pool.
    fn is_hit(&self) -> bool;

    /// Borrowed value when present.
    fn get(&self) -> Option<&(dyn Any + Send + Sync)>;

    /// Replaces the stored value.
    fn set(&mut self, value: Arc<dyn Any + Send + Sync>);

    /// Sets absolute expiry from now, or clears expiry when `None`.
    fn expires_after(&mut self, ttl: Option<Duration>);

    /// Returns `true` when the item is past its expiry.
    fn is_expired(&self) -> bool;
}

/// In-memory [`CacheItem`] used by [`crate::ArrayAdapter`].
#[derive(Clone)]
pub struct ArrayCacheItem {
    key: String,
    hit: bool,
    value: Option<Arc<dyn Any + Send + Sync>>,
    expires_at: Option<Instant>,
}

impl ArrayCacheItem {
    /// Creates a miss item for `key`.
    #[must_use]
    pub fn miss(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            hit: false,
            value: None,
            expires_at: None,
        }
    }

    /// Creates a hit item for `key` with `value`.
    #[must_use]
    pub fn hit(key: impl Into<String>, value: Arc<dyn Any + Send + Sync>) -> Self {
        Self {
            key: key.into(),
            hit: true,
            value: Some(value),
            expires_at: None,
        }
    }

    pub(crate) const fn with_expiry(mut self, expires_at: Option<Instant>) -> Self {
        self.expires_at = expires_at;
        self
    }

    pub(crate) fn into_stored(
        self,
    ) -> (String, Option<Arc<dyn Any + Send + Sync>>, Option<Instant>) {
        (self.key, self.value, self.expires_at)
    }
}

impl CacheItem for ArrayCacheItem {
    fn key(&self) -> &str {
        &self.key
    }

    fn is_hit(&self) -> bool {
        self.hit && !self.is_expired()
    }

    fn get(&self) -> Option<&(dyn Any + Send + Sync)> {
        if self.is_expired() {
            return None;
        }
        self.value.as_deref()
    }

    fn set(&mut self, value: Arc<dyn Any + Send + Sync>) {
        self.value = Some(value);
        self.hit = true;
    }

    fn expires_after(&mut self, ttl: Option<Duration>) {
        self.expires_at = ttl.map(|duration| Instant::now() + duration);
    }

    fn is_expired(&self) -> bool {
        self.expires_at
            .is_some_and(|deadline| Instant::now() >= deadline)
    }
}
