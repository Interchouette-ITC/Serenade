//! Optional Messenger bridge for due schedules.

use std::marker::PhantomData;

use serenade_messenger::{Command, MessageBus};

use crate::{DueJob, ScheduleHandler, SchedulerError};

/// Builds a [`Command`] from a [`DueJob`] and dispatches it on a [`MessageBus`].
pub struct CommandOnDue<C, F>
where
    C: Command,
    F: Fn(&DueJob) -> C + Send + Sync,
{
    bus: MessageBus,
    factory: F,
    _command: PhantomData<C>,
}

impl<C, F> CommandOnDue<C, F>
where
    C: Command,
    F: Fn(&DueJob) -> C + Send + Sync,
{
    /// Uses `bus` and `factory` for every due fire.
    #[must_use]
    pub const fn new(bus: MessageBus, factory: F) -> Self {
        Self {
            bus,
            factory,
            _command: PhantomData,
        }
    }
}

impl<C, F> ScheduleHandler for CommandOnDue<C, F>
where
    C: Command,
    F: Fn(&DueJob) -> C + Send + Sync,
{
    fn on_due(&self, job: &DueJob) -> Result<(), SchedulerError> {
        let command = (self.factory)(job);
        self.bus
            .dispatch_command(&command)
            .map_err(|error| SchedulerError::Handler {
                id: job.id().to_owned(),
                message: error.to_string(),
            })
    }
}
