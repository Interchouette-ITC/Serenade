//! Compile pass that collects tagged commands into an [`Application`].

use std::sync::Arc;

use serenade_di::{CompilePass, ContainerBuilder, DiError, Reference, ServiceDefinition};

use crate::{Application, Command};

/// Service tag applied to console command definitions.
pub const COMMAND_TAG: &str = "console.command";

/// Container id of the compiled [`Application`].
pub const APPLICATION_SERVICE: &str = "console.application";

/// Newtype so command trait objects can be stored in the container.
#[derive(Clone)]
pub struct CommandService(pub Arc<dyn Command>);

/// Registers [`APPLICATION_SERVICE`] from services tagged [`COMMAND_TAG`].
#[derive(Debug, Default)]
pub struct RegisterCommandsPass;

impl CompilePass for RegisterCommandsPass {
    fn name(&self) -> &'static str {
        "register_console_commands"
    }

    fn process(&self, builder: &mut ContainerBuilder) -> Result<(), DiError> {
        let tagged: Vec<String> = builder
            .definitions()
            .iter()
            .filter(|definition| definition.tags().iter().any(|tag| tag == COMMAND_TAG))
            .map(|definition| definition.id().to_owned())
            .collect();

        let dependencies = tagged
            .iter()
            .map(|id| Reference::from(id.clone()))
            .collect();
        builder.register(
            ServiceDefinition::new(APPLICATION_SERVICE).with_dependencies(dependencies),
            move |container| {
                let mut application = Application::new();
                for id in &tagged {
                    let wrapped = container.get_as::<CommandService>(id)?;
                    application.add(Arc::clone(&wrapped.0));
                }
                Ok(Box::new(application))
            },
        )
    }
}
