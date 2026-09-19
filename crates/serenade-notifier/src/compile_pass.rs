//! DI tag and compile pass for the default notifier.

use std::sync::Arc;

use serenade_di::{CompilePass, ContainerBuilder, DiError, ServiceDefinition};

use crate::{Notification, NotifierError, NullTransport, Transport};

/// Service tag applied to notifier transport definitions.
pub const NOTIFIER_TRANSPORT_TAG: &str = "notifier.transport";

/// Container id of the default notifier when registered by the pass.
pub const DEFAULT_NOTIFIER_SERVICE: &str = "notifier";

/// Newtype so transport trait objects can be stored in the container.
#[derive(Clone)]
pub struct NotifierService(pub Arc<dyn Transport>);

impl NotifierService {
    /// Sends through the wrapped transport.
    ///
    /// # Errors
    ///
    /// Propagates [`NotifierError`] from the transport.
    pub fn send(&self, notification: &Notification) -> Result<(), NotifierError> {
        self.0.send(notification)
    }
}

/// Seeds [`DEFAULT_NOTIFIER_SERVICE`] with a [`NullTransport`] when missing.
#[derive(Debug, Default)]
pub struct RegisterDefaultNotifierPass;

impl CompilePass for RegisterDefaultNotifierPass {
    fn name(&self) -> &'static str {
        "register_default_notifier"
    }

    fn process(&self, builder: &mut ContainerBuilder) -> Result<(), DiError> {
        let has_default = builder
            .definitions()
            .iter()
            .any(|definition| definition.id() == DEFAULT_NOTIFIER_SERVICE);
        if has_default {
            return Ok(());
        }

        builder.register(
            ServiceDefinition::new(DEFAULT_NOTIFIER_SERVICE).with_tag(NOTIFIER_TRANSPORT_TAG),
            |_container| {
                Ok(Box::new(NotifierService(
                    Arc::new(NullTransport::new()) as Arc<dyn Transport>
                )))
            },
        )
    }
}
