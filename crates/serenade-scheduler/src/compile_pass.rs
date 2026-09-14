//! DI tag and compile pass for the default scheduler.

use std::sync::{Arc, Mutex};

use serenade_di::{CompilePass, ContainerBuilder, DiError, ServiceDefinition};

use crate::{Clock, DueJob, HandlerMap, Scheduler, SchedulerError};

/// Service tag applied to scheduler definitions.
pub const SCHEDULER_TAG: &str = "scheduler";

/// Container id of the default scheduler when registered by the pass.
pub const DEFAULT_SCHEDULER_SERVICE: &str = "scheduler";

/// Shared [`Scheduler`] plus [`HandlerMap`] for apps and console commands.
#[derive(Clone, Default)]
pub struct SchedulerService {
    scheduler: Arc<Mutex<Scheduler>>,
    handlers: Arc<Mutex<HandlerMap>>,
}

impl SchedulerService {
    /// Empty scheduler and handler map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a schedule definition.
    ///
    /// # Errors
    ///
    /// Propagates [`Scheduler::add`] errors.
    ///
    /// # Panics
    ///
    /// Panics when an internal mutex is poisoned.
    pub fn add(&self, schedule: crate::Schedule, clock: &dyn Clock) -> Result<(), SchedulerError> {
        self.scheduler
            .lock()
            .expect("serenade-scheduler SchedulerService mutex poisoned")
            .add(schedule, clock)
    }

    /// Registers a sync handler for a schedule id.
    ///
    /// # Errors
    ///
    /// Propagates [`HandlerMap::insert`] errors.
    ///
    /// # Panics
    ///
    /// Panics when an internal mutex is poisoned.
    pub fn insert_handler(
        &self,
        id: impl Into<String>,
        handler: Arc<dyn crate::ScheduleHandler>,
    ) -> Result<(), SchedulerError> {
        self.handlers
            .lock()
            .expect("serenade-scheduler SchedulerService mutex poisoned")
            .insert(id, handler)
    }

    /// Earliest next due time across all schedules.
    ///
    /// # Panics
    ///
    /// Panics when an internal mutex is poisoned.
    #[must_use]
    pub fn next_wake(&self) -> Option<std::time::SystemTime> {
        self.scheduler
            .lock()
            .expect("serenade-scheduler SchedulerService mutex poisoned")
            .next_wake()
    }

    /// Whether no schedules are registered.
    ///
    /// # Panics
    ///
    /// Panics when an internal mutex is poisoned.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.scheduler
            .lock()
            .expect("serenade-scheduler SchedulerService mutex poisoned")
            .is_empty()
    }

    /// Ticks due jobs and dispatches each through the handler map.
    ///
    /// # Errors
    ///
    /// Propagates missing-handler and handler failures after due jobs are collected.
    ///
    /// # Panics
    ///
    /// Panics when an internal mutex is poisoned.
    pub fn tick_and_dispatch(&self, clock: &dyn Clock) -> Result<Vec<DueJob>, SchedulerError> {
        let due = self
            .scheduler
            .lock()
            .expect("serenade-scheduler SchedulerService mutex poisoned")
            .tick(clock);
        {
            let handlers = self
                .handlers
                .lock()
                .expect("serenade-scheduler SchedulerService mutex poisoned");
            for job in &due {
                handlers.dispatch(job)?;
            }
        }
        Ok(due)
    }
}

/// Seeds [`DEFAULT_SCHEDULER_SERVICE`] with an empty [`SchedulerService`] when missing.
#[derive(Debug, Default)]
pub struct RegisterDefaultSchedulerPass;

impl CompilePass for RegisterDefaultSchedulerPass {
    fn name(&self) -> &'static str {
        "register_default_scheduler"
    }

    fn process(&self, builder: &mut ContainerBuilder) -> Result<(), DiError> {
        let has_default = builder
            .definitions()
            .iter()
            .any(|definition| definition.id() == DEFAULT_SCHEDULER_SERVICE);
        if has_default {
            return Ok(());
        }

        // `expect`: default id cannot collide after the has_default guard above.
        builder
            .register(
                ServiceDefinition::new(DEFAULT_SCHEDULER_SERVICE).with_tag(SCHEDULER_TAG),
                |_container| Ok(Box::new(SchedulerService::new())),
            )
            .expect("default scheduler id is unique");

        Ok(())
    }
}
