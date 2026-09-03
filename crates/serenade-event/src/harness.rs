//! Helpers for asserting dispatched events in tests.

use std::sync::{Arc, Mutex};

use crate::{Event, EventError, EventSubscriber};

/// Records event names in dispatch order for a single event name.
#[derive(Clone)]
pub struct RecordingSubscriber {
    event_name: &'static str,
    priority: i32,
    names: Arc<Mutex<Vec<&'static str>>>,
}

impl RecordingSubscriber {
    /// Creates a recorder for `event_name`.
    #[must_use]
    pub const fn new(event_name: &'static str, names: Arc<Mutex<Vec<&'static str>>>) -> Self {
        Self {
            event_name,
            priority: 0,
            names,
        }
    }

    /// Sets subscriber priority (higher runs first).
    #[must_use]
    pub const fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
}

impl EventSubscriber for RecordingSubscriber {
    fn event_name(&self) -> &'static str {
        self.event_name
    }

    fn priority(&self) -> i32 {
        self.priority
    }

    fn handle(&self, event: &dyn Event) -> Result<(), EventError> {
        self.names
            .lock()
            .map_err(|_| EventError::Subscriber {
                subscriber: self.event_name.to_owned(),
                event: event.name(),
                message: "lock poisoned".to_owned(),
            })?
            .push(event.name());
        Ok(())
    }
}

/// Asserts recorded event names equal `expected` in order.
///
/// # Panics
///
/// Panics when the sequences differ.
pub fn assert_dispatched(recorded: &[&str], expected: &[&str]) {
    assert_eq!(recorded, expected);
}
