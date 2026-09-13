//! Process-local [`RateLimiterStorage`].

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::{Policy, RateLimit, RateLimiterError, RateLimiterStorage, WindowState, validate_key};

#[derive(Debug)]
struct Entry {
    state: Option<WindowState>,
    expires_at: Option<Instant>,
}

impl Entry {
    fn is_expired(&self, now: Instant) -> bool {
        self.expires_at.is_some_and(|at| at <= now)
    }
}

/// In-process rate-limiter storage backed by a [`Mutex`] + [`HashMap`].
#[derive(Debug, Default)]
pub struct InMemoryRateLimiterStorage {
    inner: Mutex<HashMap<String, Entry>>,
}

impl InMemoryRateLimiterStorage {
    /// Empty storage.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn with_map<T>(&self, f: impl FnOnce(&mut HashMap<String, Entry>) -> T) -> T {
        let mut guard = self
            .inner
            .lock()
            .expect("serenade-rate-limiter InMemoryRateLimiterStorage mutex poisoned");
        f(&mut guard)
    }
}

impl RateLimiterStorage for InMemoryRateLimiterStorage {
    fn consume(
        &self,
        id: &str,
        policy: &Policy,
        tokens: u32,
        now: Instant,
        ttl: Option<Duration>,
    ) -> Result<RateLimit, RateLimiterError> {
        validate_key(id)?;
        self.with_map(|map| {
            let entry = map.entry(id.to_owned()).or_insert_with(|| Entry {
                state: None,
                expires_at: None,
            });
            if entry.is_expired(now) {
                entry.state = None;
                entry.expires_at = None;
            }
            let result = policy.consume(&mut entry.state, tokens, now)?;
            entry.expires_at = ttl.map(|duration| now + duration);
            Ok(result)
        })
    }

    fn reset(&self, id: &str) -> Result<(), RateLimiterError> {
        validate_key(id)?;
        self.with_map(|map| {
            map.remove(id);
            Ok(())
        })
    }
}
