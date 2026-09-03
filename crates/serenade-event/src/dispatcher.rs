//! In-process synchronous event dispatcher.

use std::sync::Arc;

use crate::{Event, EventError, EventSubscriber};

/// Dispatches events to subscribers sorted by priority (high first).
///
/// # Examples
///
/// ```
/// use std::sync::{Arc, Mutex};
/// use serenade_event::{
///     assert_dispatched, Event, EventDispatcher, RecordingSubscriber,
/// };
///
/// struct Ping;
///
/// impl Event for Ping {
///     fn name(&self) -> &'static str {
///         "demo.ping"
///     }
/// }
///
/// let names = Arc::new(Mutex::new(Vec::new()));
/// let mut dispatcher = EventDispatcher::new();
/// dispatcher.add(Arc::new(RecordingSubscriber::new("demo.ping", Arc::clone(&names))));
/// dispatcher.dispatch(&Ping).expect("dispatch");
/// assert_dispatched(&names.lock().expect("lock"), &["demo.ping"]);
/// ```
#[derive(Clone, Default)]
pub struct EventDispatcher {
    subscribers: Vec<Arc<dyn EventSubscriber>>,
}

impl EventDispatcher {
    /// Creates an empty dispatcher.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a subscriber. Call [`Self::sort`] before dispatch if adding after construction.
    pub fn add(&mut self, subscriber: Arc<dyn EventSubscriber>) {
        self.subscribers.push(subscriber);
        self.sort();
    }

    /// Sorts subscribers so higher priority runs first.
    pub fn sort(&mut self) {
        self.subscribers
            .sort_by_key(|subscriber| std::cmp::Reverse(subscriber.priority()));
    }

    /// Dispatches `event` to every subscriber whose [`EventSubscriber::event_name`] matches.
    ///
    /// All matching subscribers run even if one fails. The first error is returned.
    ///
    /// # Errors
    ///
    /// Returns the first [`EventError`] from a subscriber.
    pub fn dispatch(&self, event: &dyn Event) -> Result<(), EventError> {
        let mut first_error = None;
        for subscriber in &self.subscribers {
            if subscriber.event_name() != event.name() {
                continue;
            }
            if let Err(error) = subscriber.handle(event) {
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
        first_error.map_or(Ok(()), Err)
    }

    /// Number of registered subscribers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.subscribers.len()
    }

    /// Returns whether no subscribers are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.subscribers.is_empty()
    }
}
