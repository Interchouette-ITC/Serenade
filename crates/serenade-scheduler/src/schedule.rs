//! Named schedule definitions and due jobs.

use std::time::SystemTime;

use crate::{SchedulerError, Trigger, validate_id};

/// Named recurring schedule.
#[derive(Clone, Debug)]
pub struct Schedule {
    id: String,
    trigger: Trigger,
}

impl Schedule {
    /// Builds a schedule with `id` and `trigger`.
    ///
    /// # Errors
    ///
    /// Returns [`SchedulerError::InvalidId`] when `id` is empty.
    pub fn new(id: impl Into<String>, trigger: Trigger) -> Result<Self, SchedulerError> {
        let id = id.into();
        validate_id(&id)?;
        Ok(Self { id, trigger })
    }

    /// Schedule id.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Trigger definition.
    #[must_use]
    pub const fn trigger(&self) -> &Trigger {
        &self.trigger
    }

    pub(crate) fn into_parts(self) -> (String, Trigger) {
        (self.id, self.trigger)
    }
}

/// A schedule that became due on a [`crate::Scheduler::tick`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DueJob {
    id: String,
    scheduled_for: SystemTime,
}

impl DueJob {
    pub(crate) const fn new(id: String, scheduled_for: SystemTime) -> Self {
        Self { id, scheduled_for }
    }

    /// Schedule id that fired.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Instant this fire was armed for.
    #[must_use]
    pub const fn scheduled_for(&self) -> SystemTime {
        self.scheduled_for
    }
}
