//! Bounded in-memory profile storage.

#![allow(clippy::significant_drop_tightening)]

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use crate::data::{LogLine, ProfileData, QueryEvent};

/// Thread-safe ring of recent [`ProfileData`] values.
#[derive(Debug)]
pub struct ProfileStore {
    capacity: usize,
    inner: Mutex<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    order: VecDeque<String>,
    by_token: HashMap<String, ProfileData>,
}

impl ProfileStore {
    /// Store that keeps at most `capacity` profiles.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            inner: Mutex::new(Inner::default()),
        }
    }

    /// Inserts or replaces a finished profile.
    pub fn insert(&self, profile: ProfileData) {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let token = profile.token.clone();
        if guard.by_token.insert(token.clone(), profile).is_none() {
            guard.order.push_back(token);
        }
        while guard.order.len() > self.capacity {
            if let Some(old) = guard.order.pop_front() {
                guard.by_token.remove(&old);
            }
        }
    }

    /// Looks up a profile by token.
    #[must_use]
    pub fn get(&self, token: &str) -> Option<ProfileData> {
        let guard = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.by_token.get(token).cloned()
    }

    /// Newest-first list of retained profiles.
    #[must_use]
    pub fn list(&self) -> Vec<ProfileData> {
        let guard = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard
            .order
            .iter()
            .rev()
            .filter_map(|token| guard.by_token.get(token).cloned())
            .collect()
    }

    /// Appends a query event to an in-flight or stored profile.
    pub fn push_query(&self, token: &str, event: QueryEvent) {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(profile) = guard.by_token.get_mut(token) {
            profile.queries.push(event);
        }
    }

    /// Appends a log line to a profile.
    pub fn push_log(&self, token: &str, line: LogLine) {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(profile) = guard.by_token.get_mut(token) {
            profile.logs.push(line);
        }
    }

    /// Ensures a stub profile exists so mid-request query/log pushes have a home.
    pub fn ensure(&self, profile: ProfileData) {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let token = profile.token.clone();
        if let Some(existing) = guard.by_token.get_mut(&token) {
            existing.method = profile.method;
            existing.path = profile.path;
            existing.route = profile.route;
        } else {
            guard.by_token.insert(token.clone(), profile);
            guard.order.push_back(token);
            while guard.order.len() > self.capacity {
                if let Some(old) = guard.order.pop_front() {
                    guard.by_token.remove(&old);
                }
            }
        }
    }

    /// Sets the matched route name when known.
    pub fn set_route(&self, token: &str, route: String) {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(profile) = guard.by_token.get_mut(token) {
            profile.route = Some(route);
        }
    }

    /// Updates status and duration after the controller returns.
    pub fn finish(&self, token: &str, status: u16, duration: std::time::Duration) {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(profile) = guard.by_token.get_mut(token) {
            profile.status = status;
            profile.duration = duration;
        }
    }

    /// Number of retained profiles.
    #[must_use]
    pub fn len(&self) -> usize {
        let guard = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.order.len()
    }

    /// Whether the store is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
