//! Sync handlers for due schedules.

use std::collections::HashMap;
use std::sync::Arc;

use crate::{DueJob, SchedulerError, validate_id};

/// Invoked when a named schedule becomes due.
pub trait ScheduleHandler: Send + Sync {
    /// Handles one [`DueJob`].
    ///
    /// # Errors
    ///
    /// Returns [`SchedulerError::Handler`] (or another variant) when work fails.
    fn on_due(&self, job: &DueJob) -> Result<(), SchedulerError>;
}

/// Maps schedule ids to [`ScheduleHandler`]s.
#[derive(Clone, Default)]
pub struct HandlerMap {
    handlers: HashMap<String, Arc<dyn ScheduleHandler>>,
}

impl HandlerMap {
    /// Empty map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of registered handlers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    /// Whether no handlers are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }

    /// Registers `handler` for schedule `id`.
    ///
    /// # Errors
    ///
    /// Returns [`SchedulerError::InvalidId`] when `id` is empty or already present.
    pub fn insert(
        &mut self,
        id: impl Into<String>,
        handler: Arc<dyn ScheduleHandler>,
    ) -> Result<(), SchedulerError> {
        let id = id.into();
        validate_id(&id)?;
        if self.handlers.contains_key(&id) {
            return Err(SchedulerError::InvalidId {
                id,
                message: "schedule handler id already registered".to_owned(),
            });
        }
        self.handlers.insert(id, handler);
        Ok(())
    }

    /// Removes a handler by id. Returns whether it was present.
    pub fn remove(&mut self, id: &str) -> bool {
        self.handlers.remove(id).is_some()
    }

    /// Dispatches `job` to its registered handler.
    ///
    /// # Errors
    ///
    /// Returns [`SchedulerError::MissingHandler`] when no handler is registered,
    /// or propagates handler failures.
    pub fn dispatch(&self, job: &DueJob) -> Result<(), SchedulerError> {
        let Some(handler) = self.handlers.get(job.id()) else {
            return Err(SchedulerError::MissingHandler {
                id: job.id().to_owned(),
            });
        };
        handler.on_due(job)
    }
}

/// Closure-backed [`ScheduleHandler`].
pub struct FnScheduleHandler<F>
where
    F: Fn(&DueJob) -> Result<(), SchedulerError> + Send + Sync,
{
    inner: F,
}

impl<F> FnScheduleHandler<F>
where
    F: Fn(&DueJob) -> Result<(), SchedulerError> + Send + Sync,
{
    /// Wraps `inner`.
    #[must_use]
    pub const fn new(inner: F) -> Self {
        Self { inner }
    }
}

impl<F> ScheduleHandler for FnScheduleHandler<F>
where
    F: Fn(&DueJob) -> Result<(), SchedulerError> + Send + Sync,
{
    fn on_due(&self, job: &DueJob) -> Result<(), SchedulerError> {
        (self.inner)(job)
    }
}
