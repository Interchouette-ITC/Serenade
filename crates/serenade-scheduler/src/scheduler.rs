//! Tick-based schedule runner.

use std::time::SystemTime;

use crate::{Clock, DueJob, Schedule, SchedulerError, Trigger};

#[derive(Debug)]
struct Entry {
    id: String,
    trigger: Trigger,
    next: SystemTime,
}

/// Collects schedules and reports due work on [`Self::tick`].
///
/// Does not sleep. Production loops call `tick`, handle [`DueJob`]s, then sleep
/// until [`Self::next_wake`].
#[derive(Debug, Default)]
pub struct Scheduler {
    entries: Vec<Entry>,
}

impl Scheduler {
    /// Empty scheduler.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `schedule`, arming the first due time from `clock.now()`.
    ///
    /// # Errors
    ///
    /// Returns [`SchedulerError::InvalidId`] when the id is empty or already registered.
    pub fn add(&mut self, schedule: Schedule, clock: &dyn Clock) -> Result<(), SchedulerError> {
        validate_id(schedule.id())?;
        if self.entries.iter().any(|entry| entry.id == schedule.id()) {
            return Err(SchedulerError::InvalidId {
                id: schedule.id().to_owned(),
                message: "schedule id already registered".to_owned(),
            });
        }
        let now = clock.now();
        let next = schedule.trigger().first_due(now);
        let (id, trigger) = schedule.into_parts();
        self.entries.push(Entry { id, trigger, next });
        Ok(())
    }

    /// Number of registered schedules.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no schedules are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Earliest next due time across all schedules.
    #[must_use]
    pub fn next_wake(&self) -> Option<SystemTime> {
        self.entries.iter().map(|entry| entry.next).min()
    }

    /// Returns every schedule whose next due time is `<= clock.now()`, then re-arms them.
    pub fn tick(&mut self, clock: &dyn Clock) -> Vec<DueJob> {
        let now = clock.now();
        let mut due = Vec::new();
        for entry in &mut self.entries {
            if entry.next <= now {
                due.push(DueJob::new(entry.id.clone(), entry.next));
                entry.next = entry.trigger.next_after(entry.next, now);
            }
        }
        due
    }

    /// Removes a schedule by id. Returns whether it was present.
    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|entry| entry.id != id);
        self.entries.len() != before
    }
}

/// Rejects empty schedule ids.
///
/// # Errors
///
/// Returns [`SchedulerError::InvalidId`] when `id` is empty.
pub fn validate_id(id: &str) -> Result<(), SchedulerError> {
    if id.is_empty() {
        return Err(SchedulerError::InvalidId {
            id: id.to_owned(),
            message: "id must not be empty".to_owned(),
        });
    }
    Ok(())
}
