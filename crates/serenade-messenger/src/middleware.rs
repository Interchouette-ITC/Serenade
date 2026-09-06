//! Middleware pipeline for the sync message bus.

use crate::MessengerError;

/// Whether the bus is dispatching a command or an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchKind {
    /// Single-handler command.
    Command,
    /// Fan-out event.
    Event,
}

/// Context passed to each middleware for one dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DispatchContext {
    /// [`crate::Message::NAME`] for the payload.
    pub message_name: &'static str,
    /// Command vs event dispatch.
    pub kind: DispatchKind,
}

/// Onion layer around handler execution.
///
/// Middlewares run in registration order (first registered is outermost). Call `next` to
/// continue toward the handler (or reject without calling `next`).
pub trait Middleware: Send + Sync {
    /// Runs this layer, optionally calling `next`.
    ///
    /// # Errors
    ///
    /// Return [`MessengerError`] to abort the dispatch (after or instead of `next`).
    fn handle(
        &self,
        ctx: &DispatchContext,
        next: &dyn Fn() -> Result<(), MessengerError>,
    ) -> Result<(), MessengerError>;
}

/// Sink used by [`LoggingMiddleware`] (apps plug `tracing` / files later via #57).
pub trait LogSink: Send + Sync {
    /// Records that a message is entering the pipeline.
    fn record(&self, ctx: &DispatchContext);
}

/// Middleware that records each dispatch through a [`LogSink`].
#[derive(Debug, Clone)]
pub struct LoggingMiddleware<S> {
    sink: S,
}

impl<S> LoggingMiddleware<S> {
    /// Creates a logging layer around `sink`.
    #[must_use]
    pub const fn new(sink: S) -> Self {
        Self { sink }
    }
}

impl<S: LogSink> Middleware for LoggingMiddleware<S> {
    fn handle(
        &self,
        ctx: &DispatchContext,
        next: &dyn Fn() -> Result<(), MessengerError>,
    ) -> Result<(), MessengerError> {
        self.sink.record(ctx);
        next()
    }
}

/// Hook used by [`ValidationMiddleware`] before the handler runs.
pub trait ValidateHook: Send + Sync {
    /// Returns `Ok(())` when the message may proceed.
    ///
    /// # Errors
    ///
    /// Reject invalid messages (typically [`MessengerError::Rejected`]).
    fn validate(&self, ctx: &DispatchContext) -> Result<(), MessengerError>;
}

/// Middleware that runs a [`ValidateHook`] before calling `next`.
#[derive(Debug, Clone)]
pub struct ValidationMiddleware<V> {
    validator: V,
}

impl<V> ValidationMiddleware<V> {
    /// Creates a validation layer around `validator`.
    #[must_use]
    pub const fn new(validator: V) -> Self {
        Self { validator }
    }
}

impl<V: ValidateHook> Middleware for ValidationMiddleware<V> {
    fn handle(
        &self,
        ctx: &DispatchContext,
        next: &dyn Fn() -> Result<(), MessengerError>,
    ) -> Result<(), MessengerError> {
        self.validator.validate(ctx)?;
        next()
    }
}
