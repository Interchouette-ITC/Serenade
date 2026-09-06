//! In-process synchronous message bus.

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use crate::middleware::{DispatchContext, DispatchKind, Middleware};
use crate::{Command, CommandHandler, Event, EventHandler, MessengerError};

type ErasedHandler = Arc<dyn Fn(&dyn Any) -> Result<(), MessengerError> + Send + Sync>;

fn downcast_message<'msg, T: 'static>(
    name: &'static str,
    payload: &'msg dyn Any,
    kind: &str,
) -> Result<&'msg T, MessengerError> {
    payload
        .downcast_ref::<T>()
        .ok_or_else(|| MessengerError::Handler {
            name,
            message: format!("{kind} type mismatch"),
        })
}

fn run_pipeline(
    middlewares: &[Arc<dyn Middleware>],
    ctx: &DispatchContext<'_>,
    terminal: &dyn Fn() -> Result<(), MessengerError>,
) -> Result<(), MessengerError> {
    fn invoke(
        index: usize,
        middlewares: &[Arc<dyn Middleware>],
        ctx: &DispatchContext<'_>,
        terminal: &dyn Fn() -> Result<(), MessengerError>,
    ) -> Result<(), MessengerError> {
        if index >= middlewares.len() {
            return terminal();
        }
        let next = || invoke(index + 1, middlewares, ctx, terminal);
        middlewares[index].handle(ctx, &next)
    }
    invoke(0, middlewares, ctx, terminal)
}

/// Dispatches commands (one handler) and events (fan-out) in-process.
///
/// # Examples
///
/// ```
/// use serenade_messenger::{
///     Command, CommandHandler, Message, MessageBus, MessengerError,
/// };
///
/// struct EnqueueJob;
///
/// impl Message for EnqueueJob {
///     const NAME: &'static str = "job.enqueue";
/// }
///
/// impl Command for EnqueueJob {}
///
/// struct EnqueueJobHandler;
///
/// impl CommandHandler<EnqueueJob> for EnqueueJobHandler {
///     fn handle(&self, _command: &EnqueueJob) -> Result<(), MessengerError> {
///         Ok(())
///     }
/// }
///
/// let mut bus = MessageBus::new();
/// bus.register_command(EnqueueJobHandler).expect("register");
/// bus.dispatch_command(&EnqueueJob).expect("dispatch");
/// ```
#[derive(Clone, Default)]
pub struct MessageBus {
    commands: HashMap<&'static str, ErasedHandler>,
    events: HashMap<&'static str, Vec<ErasedHandler>>,
    middlewares: Vec<Arc<dyn Middleware>>,
}

impl MessageBus {
    /// Creates an empty bus.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a middleware layer (first registered runs outermost).
    pub fn add_middleware(&mut self, middleware: impl Middleware + 'static) {
        self.middlewares.push(Arc::new(middleware));
    }

    /// Number of registered middleware layers.
    #[must_use]
    pub fn middleware_count(&self) -> usize {
        self.middlewares.len()
    }

    /// Registers the sole command handler for `C::NAME`.
    ///
    /// # Errors
    ///
    /// Returns [`MessengerError::DuplicateCommand`] when a handler already exists.
    pub fn register_command<C, H>(&mut self, handler: H) -> Result<(), MessengerError>
    where
        C: Command,
        H: CommandHandler<C> + 'static,
    {
        let name = C::NAME;
        if self.commands.contains_key(name) {
            return Err(MessengerError::DuplicateCommand { name });
        }
        let handler = Arc::new(handler);
        let erased: ErasedHandler = Arc::new(move |payload: &dyn Any| {
            let command = downcast_message::<C>(name, payload, "command")?;
            handler.handle(command)
        });
        self.commands.insert(name, erased);
        Ok(())
    }

    /// Appends an event handler for `E::NAME` (fan-out; order is registration order).
    pub fn register_event<E, H>(&mut self, handler: H)
    where
        E: Event,
        H: EventHandler<E> + 'static,
    {
        let name = E::NAME;
        let handler = Arc::new(handler);
        let erased: ErasedHandler = Arc::new(move |payload: &dyn Any| {
            let event = downcast_message::<E>(name, payload, "event")?;
            handler.handle(event)
        });
        self.events.entry(name).or_default().push(erased);
    }

    /// Dispatches `command` through the middleware pipeline to its handler.
    ///
    /// # Errors
    ///
    /// Returns [`MessengerError::UnknownCommand`] when no handler is registered, a
    /// middleware rejection, or the handler's [`MessengerError`].
    pub fn dispatch_command<C: Command>(&self, command: &C) -> Result<(), MessengerError> {
        let name = C::NAME;
        let Some(handler) = self.commands.get(name) else {
            return Err(MessengerError::UnknownCommand { name });
        };
        let ctx = DispatchContext {
            message_name: name,
            kind: DispatchKind::Command,
            payload: command,
        };
        run_pipeline(&self.middlewares, &ctx, &|| handler(command as &dyn Any))
    }

    /// Dispatches `event` through middleware, then to every registered handler for `E::NAME`.
    ///
    /// Missing handlers are a no-op after middleware. All matching handlers run even if one
    /// fails; the first error is returned.
    ///
    /// # Errors
    ///
    /// Returns a middleware rejection or the first [`MessengerError`] from a handler.
    pub fn dispatch_event<E: Event>(&self, event: &E) -> Result<(), MessengerError> {
        let name = E::NAME;
        let ctx = DispatchContext {
            message_name: name,
            kind: DispatchKind::Event,
            payload: event,
        };
        run_pipeline(&self.middlewares, &ctx, &|| {
            let Some(handlers) = self.events.get(name) else {
                return Ok(());
            };
            let mut first_error = None;
            for handler in handlers {
                if let Err(error) = handler(event as &dyn Any) {
                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                }
            }
            first_error.map_or(Ok(()), Err)
        })
    }

    /// Number of registered command handlers.
    #[must_use]
    pub fn command_count(&self) -> usize {
        self.commands.len()
    }

    /// Number of registered event handler callbacks (across all event names).
    #[must_use]
    pub fn event_handler_count(&self) -> usize {
        self.events.values().map(Vec::len).sum()
    }

    /// Returns `true` when no handlers are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty() && self.events.is_empty()
    }
}

#[cfg(test)]
mod downcast_tests {
    use super::downcast_message;
    use crate::MessengerError;

    #[test]
    fn downcast_message_rejects_wrong_type() {
        let payload = 42_u32;
        let err =
            downcast_message::<String>("demo.msg", &payload, "command").expect_err("type mismatch");
        assert_eq!(
            err,
            MessengerError::Handler {
                name: "demo.msg",
                message: "command type mismatch".to_owned(),
            }
        );
    }

    #[test]
    fn downcast_message_accepts_matching_type() {
        let payload = String::from("ok");
        let got = downcast_message::<String>("demo.msg", &payload, "event").expect("match");
        assert_eq!(got, "ok");
    }
}
