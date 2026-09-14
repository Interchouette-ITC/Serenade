//! Injectable clocks for the scheduler runner.

use std::sync::Mutex;
use std::time::{Duration, SystemTime};

/// Source of “now” for [`crate::Scheduler::tick`].
pub trait Clock: Send + Sync {
    /// Current instant used for due checks.
    fn now(&self) -> SystemTime;
}

/// Wall-clock [`Clock`].
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}

/// Test [`Clock`] with an explicit current time.
#[derive(Debug)]
pub struct ManualClock {
    now: Mutex<SystemTime>,
}

impl ManualClock {
    /// Starts at `now`.
    #[must_use]
    pub const fn new(now: SystemTime) -> Self {
        Self {
            now: Mutex::new(now),
        }
    }

    /// Advances the clock by `delta`.
    ///
    /// # Panics
    ///
    /// Panics when the internal mutex is poisoned.
    pub fn advance(&self, delta: Duration) {
        let mut guard = self
            .now
            .lock()
            .expect("serenade-scheduler ManualClock mutex poisoned");
        *guard += delta;
    }

    /// Sets the clock to an absolute instant.
    ///
    /// # Panics
    ///
    /// Panics when the internal mutex is poisoned.
    pub fn set(&self, now: SystemTime) {
        let mut guard = self
            .now
            .lock()
            .expect("serenade-scheduler ManualClock mutex poisoned");
        *guard = now;
    }
}

impl Clock for ManualClock {
    fn now(&self) -> SystemTime {
        *self
            .now
            .lock()
            .expect("serenade-scheduler ManualClock mutex poisoned")
    }
}
