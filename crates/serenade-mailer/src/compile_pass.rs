//! DI tag and compile pass for the default mailer transport.

use std::sync::Arc;

use serenade_di::{CompilePass, ContainerBuilder, DiError, ServiceDefinition};

use crate::{NullTransport, Transport};

/// Service tag applied to mail transport definitions.
pub const MAILER_TRANSPORT_TAG: &str = "mailer.transport";

/// Container id of the default mailer when registered by the pass.
pub const DEFAULT_MAILER_SERVICE: &str = "mailer";

/// Newtype so transport trait objects can be stored in the container.
#[derive(Clone)]
pub struct MailerService(pub Arc<dyn Transport>);

impl MailerService {
    /// Sends through the wrapped transport.
    ///
    /// # Errors
    ///
    /// Propagates [`crate::MailerError`] from the transport.
    pub fn send(&self, email: &crate::Email) -> Result<(), crate::MailerError> {
        self.0.send(email)
    }
}

/// Seeds [`DEFAULT_MAILER_SERVICE`] with a [`NullTransport`] when missing.
#[derive(Debug, Default)]
pub struct RegisterDefaultMailerPass;

impl CompilePass for RegisterDefaultMailerPass {
    fn name(&self) -> &'static str {
        "register_default_mailer"
    }

    fn process(&self, builder: &mut ContainerBuilder) -> Result<(), DiError> {
        let has_default = builder
            .definitions()
            .iter()
            .any(|definition| definition.id() == DEFAULT_MAILER_SERVICE);
        if has_default {
            return Ok(());
        }

        builder.register(
            ServiceDefinition::new(DEFAULT_MAILER_SERVICE).with_tag(MAILER_TRANSPORT_TAG),
            |_container| {
                Ok(Box::new(MailerService(
                    Arc::new(NullTransport::new()) as Arc<dyn Transport>
                )))
            },
        )
    }
}
