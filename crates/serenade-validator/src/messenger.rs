//! Messenger [`ValidateHook`] adapter.

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use serenade_messenger::{DispatchContext, MessengerError, ValidateHook};

use crate::{ConstraintViolationList, Validatable, Validator};

type PayloadValidateFn = dyn Fn(&dyn Any, &dyn Validator) -> ConstraintViolationList + Send + Sync;

/// Maps message names to typed [`Validatable`] payloads and rejects on violations.
///
/// # Examples
///
/// ```
/// use serenade_messenger::{
///     Command, CommandHandler, Message, MessageBus, ValidationMiddleware,
/// };
/// use serenade_validator::{
///     Constraint, ConstraintViolationList, Length, MessengerValidateHook, NotBlank,
///     RecursiveValidator, Validatable, Validator,
/// };
///
/// struct EnqueueJob {
///     code: String,
/// }
///
/// impl Message for EnqueueJob {
///     const NAME: &'static str = "job.enqueue";
/// }
/// impl Command for EnqueueJob {}
///
/// impl Validatable for EnqueueJob {
///     fn validate(&self, validator: &dyn Validator) -> ConstraintViolationList {
///         validator.validate_value(
///             &self.code,
///             "code",
///             &[&NotBlank as &dyn Constraint, &Length::new(1, 32)],
///         )
///     }
/// }
///
/// struct OkHandler;
/// impl CommandHandler<EnqueueJob> for OkHandler {
///     fn handle(&self, _: &EnqueueJob) -> Result<(), serenade_messenger::MessengerError> {
///         Ok(())
///     }
/// }
///
/// let mut hook = MessengerValidateHook::new(RecursiveValidator);
/// hook.register::<EnqueueJob>();
/// let mut bus = MessageBus::new();
/// bus.add_middleware(ValidationMiddleware::new(hook));
/// bus.register_command(OkHandler).unwrap();
/// assert!(bus
///     .dispatch_command(&EnqueueJob { code: String::new() })
///     .is_err());
/// ```
pub struct MessengerValidateHook<V> {
    validator: V,
    by_name: HashMap<&'static str, Arc<PayloadValidateFn>>,
}

impl<V> MessengerValidateHook<V> {
    /// Creates a hook around `validator`.
    #[must_use]
    pub fn new(validator: V) -> Self {
        Self {
            validator,
            by_name: HashMap::new(),
        }
    }

    /// Registers `T` under `T::NAME` when `T: Message + Validatable`.
    pub fn register<T>(&mut self)
    where
        T: serenade_messenger::Message + Validatable + 'static,
    {
        self.by_name.insert(
            T::NAME,
            Arc::new(|payload: &dyn Any, validator: &dyn Validator| {
                payload
                    .downcast_ref::<T>()
                    .map_or_else(ConstraintViolationList::new, |typed| {
                        typed.validate(validator)
                    })
            }),
        );
    }
}

impl<V: Validator> ValidateHook for MessengerValidateHook<V> {
    fn validate(&self, ctx: &DispatchContext<'_>) -> Result<(), MessengerError> {
        let Some(check) = self.by_name.get(ctx.message_name) else {
            return Ok(());
        };
        let violations = check(ctx.payload, &self.validator);
        if violations.is_empty() {
            return Ok(());
        }
        let summary = violations
            .as_slice()
            .iter()
            .map(|v| {
                if v.property_path.is_empty() {
                    v.message.clone()
                } else {
                    format!("{}: {}", v.property_path, v.message)
                }
            })
            .collect::<Vec<_>>()
            .join("; ");
        Err(MessengerError::Rejected {
            name: ctx.message_name,
            message: summary,
        })
    }
}
