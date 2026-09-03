//! Compile pass that collects tagged event subscribers into an [`EventDispatcher`].

use std::sync::Arc;

use serenade_di::{CompilePass, ContainerBuilder, DiError, Reference, ServiceDefinition};

use crate::{EventDispatcher, EventSubscriber};

/// Service tag applied to event subscriber definitions.
pub const SUBSCRIBER_TAG: &str = "event.subscriber";

/// Container id of the compiled [`EventDispatcher`].
pub const DISPATCHER_SERVICE: &str = "event_dispatcher";

/// Newtype so subscriber trait objects can be stored in the container.
#[derive(Clone)]
pub struct SubscriberService(pub Arc<dyn EventSubscriber>);

/// Registers [`DISPATCHER_SERVICE`] from services tagged [`SUBSCRIBER_TAG`].
#[derive(Debug, Default)]
pub struct RegisterEventSubscribersPass;

impl CompilePass for RegisterEventSubscribersPass {
    fn name(&self) -> &'static str {
        "register_event_subscribers"
    }

    fn process(&self, builder: &mut ContainerBuilder) -> Result<(), DiError> {
        let tagged: Vec<String> = builder
            .definitions()
            .iter()
            .filter(|definition| definition.tags().iter().any(|tag| tag == SUBSCRIBER_TAG))
            .map(|definition| definition.id().to_owned())
            .collect();

        let dependencies = tagged
            .iter()
            .map(|id| Reference::from(id.clone()))
            .collect();
        builder.register(
            ServiceDefinition::new(DISPATCHER_SERVICE).with_dependencies(dependencies),
            move |container| {
                let mut dispatcher = EventDispatcher::new();
                for id in &tagged {
                    let wrapped = container.get_as::<SubscriberService>(id)?;
                    dispatcher.add(Arc::clone(&wrapped.0));
                }
                Ok(Box::new(dispatcher))
            },
        )
    }
}
