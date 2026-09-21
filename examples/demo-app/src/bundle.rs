//! `DemoBundle` composition (`FrameworkBundle` companion for the sample app).

use std::sync::Arc;

use serenade_bundle::{BundleError, COMMAND_TAG, Extension, FRAMEWORK_BUNDLE, build_container};
use serenade_config::Config;
use serenade_console::{Command, CommandService, ConsoleError, Input};
use serenade_di::{ContainerBuilder, ServiceDefinition};
use serenade_event::{Event, EventError, EventSubscriber, SUBSCRIBER_TAG, SubscriberService};
use serenade_http::{Method, Route, RouteCollection, RouteLoader};
use serenade_kernel::BundleInterface;
use serenade_string::{pluralize, slug};
use serenade_workflow::{DefinitionBuilder, MemoryMarkingStore, Workflow};

/// Service id for the greeting string registered by [`DemoExtension`].
pub const DEMO_GREETING_SERVICE: &str = "demo.greeting";

/// Sample application bundle (`demo`).
#[derive(Clone, Copy, Debug, Default)]
pub struct DemoBundle;

impl BundleInterface for DemoBundle {
    fn name(&self) -> &'static str {
        "demo"
    }

    fn dependencies(&self) -> &'static [&'static str] {
        &[FRAMEWORK_BUNDLE]
    }
}

impl RouteLoader for DemoBundle {
    fn load(&self, collection: &mut RouteCollection) -> Result<(), serenade_http::HttpError> {
        collection.add(Route::with_method("healthz", "/healthz", Method::Get))?;
        collection.add(Route::with_method("about", "/about", Method::Get))
    }
}

/// DI extension for the `demo` package key.
#[derive(Clone, Copy, Debug, Default)]
pub struct DemoExtension;

impl Extension for DemoExtension {
    fn alias(&self) -> &'static str {
        "demo"
    }

    fn load(&self, config: &Config, builder: &mut ContainerBuilder) -> Result<(), BundleError> {
        config.apply_to(builder.parameters_mut());
        builder.register(ServiceDefinition::new(DEMO_GREETING_SERVICE), |container| {
            let name = container
                .parameters()
                .get("name")
                .map_or_else(|_| "world".to_owned(), str::to_owned);
            Ok(Box::new(format!("hello {name}")))
        })?;
        builder.register(
            ServiceDefinition::new("console.command.demo_hello").with_tag(COMMAND_TAG),
            |_| Ok(Box::new(CommandService(Arc::new(HelloCommand)))),
        )?;
        builder.register(
            ServiceDefinition::new("console.command.demo_slug").with_tag(COMMAND_TAG),
            |_| Ok(Box::new(CommandService(Arc::new(SlugCommand)))),
        )?;
        builder.register(
            ServiceDefinition::new("console.command.demo_workflow").with_tag(COMMAND_TAG),
            |_| Ok(Box::new(CommandService(Arc::new(WorkflowCommand)))),
        )?;
        builder.register(
            ServiceDefinition::new("demo.subscriber.ready").with_tag(SUBSCRIBER_TAG),
            |_| Ok(Box::new(SubscriberService(Arc::new(DemoReadySubscriber)))),
        )?;
        Ok(())
    }
}

/// Console command `demo:hello` - prints [`DEMO_GREETING_SERVICE`].
#[derive(Clone, Copy, Debug, Default)]
pub struct HelloCommand;

impl Command for HelloCommand {
    fn name(&self) -> &'static str {
        "demo:hello"
    }

    fn description(&self) -> &'static str {
        "Print the demo.greeting service value"
    }

    fn execute(&self, input: &Input) -> Result<(), ConsoleError> {
        let Some(container) = input.container() else {
            println!("demo:hello needs a container (Application::run_with)");
            return Ok(());
        };
        let greeting = container
            .get_as::<String>(DEMO_GREETING_SERVICE)
            .map_err(|error| ConsoleError::Failed(error.to_string()))?;
        println!("{greeting}");
        Ok(())
    }
}

/// Console command `demo:slug` - dogfoods [`serenade_string`].
#[derive(Clone, Copy, Debug, Default)]
pub struct SlugCommand;

impl Command for SlugCommand {
    fn name(&self) -> &'static str {
        "demo:slug"
    }

    fn description(&self) -> &'static str {
        "Slug and pluralize a phrase (serenade-string dogfood)"
    }

    fn execute(&self, input: &Input) -> Result<(), ConsoleError> {
        let phrase = input
            .args()
            .first()
            .map_or("Hello Serenade Demo", String::as_str);
        let s = slug(phrase);
        let p = pluralize("demo");
        println!("slug={s} plural={p}");
        Ok(())
    }
}

/// Console command `demo:workflow` - dogfoods [`serenade_workflow`].
#[derive(Clone, Copy, Debug, Default)]
pub struct WorkflowCommand;

impl Command for WorkflowCommand {
    fn name(&self) -> &'static str {
        "demo:workflow"
    }

    fn description(&self) -> &'static str {
        "Apply draft→published on a sample article workflow"
    }

    fn execute(&self, _input: &Input) -> Result<(), ConsoleError> {
        let marking = run_article_publish("demo-1")
            .map_err(|error| ConsoleError::Failed(error.to_string()))?;
        println!("subject=demo-1 place={marking}");
        Ok(())
    }
}

/// Builds the sample article workflow and publishes `subject_id` once.
///
/// # Errors
///
/// Returns [`serenade_workflow::WorkflowError`] when the definition or apply fails.
pub fn run_article_publish(subject_id: &str) -> Result<String, serenade_workflow::WorkflowError> {
    let definition = DefinitionBuilder::new()
        .places(["draft", "published", "archived"])
        .edge("publish", "draft", "published")
        .edge("archive", "published", "archived")
        .build()?;
    let workflow = Workflow::new(
        "demo_article",
        definition,
        Arc::new(MemoryMarkingStore::new()),
    );
    if !workflow.can(subject_id, "publish") {
        return Err(serenade_workflow::WorkflowError::NotEnabled {
            transition: "publish".into(),
        });
    }
    let next = workflow.apply(subject_id, "publish")?;
    Ok(next.places().next().unwrap_or("").to_owned())
}

/// Fired by the sample binary after boot (`demo.ready`).
#[derive(Clone, Copy, Debug, Default)]
pub struct DemoReady;

impl Event for DemoReady {
    fn name(&self) -> &'static str {
        "demo.ready"
    }
}

/// Prints a line when [`DemoReady`] is dispatched.
#[derive(Clone, Copy, Debug, Default)]
pub struct DemoReadySubscriber;

impl EventSubscriber for DemoReadySubscriber {
    fn event_name(&self) -> &'static str {
        "demo.ready"
    }

    fn handle(&self, event: &dyn Event) -> Result<(), EventError> {
        println!("subscriber heard {}", event.name());
        Ok(())
    }
}

/// Builds the sample container (tests and binaries).
///
/// # Errors
///
/// Returns [`BundleError`] when packages or extensions fail.
pub fn demo_container(
    packages_dir: Option<&std::path::Path>,
    environment: &str,
) -> Result<(Config, serenade_di::Container), BundleError> {
    build_container(
        packages_dir,
        environment,
        &[&serenade_bundle::FrameworkExtension, &DemoExtension],
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serenade_bundle::{CONSOLE_APPLICATION_SERVICE, FrameworkBundle};
    use serenade_console::{Application, Input};
    use serenade_event::DISPATCHER_SERVICE;
    use serenade_kernel::{App, Application as KernelApp, Environment};
    use serenade_string::slug;

    use super::*;

    #[test]
    fn extension_registers_greeting_command_subscriber_and_routes() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let packages = root.join("config/packages");
        let (_config, container) =
            demo_container(Some(packages.as_path()), "dev").expect("container");

        let greeting = container
            .get_as::<String>(DEMO_GREETING_SERVICE)
            .expect("greeting");
        assert_eq!(greeting.as_str(), "hello serenade-dev");

        let dispatcher = container
            .get_as::<serenade_event::EventDispatcher>(DISPATCHER_SERVICE)
            .expect("dispatcher");
        assert!(!dispatcher.is_empty());
        dispatcher.dispatch(&DemoReady).expect("dispatch");

        let mut routes = RouteCollection::new();
        DemoBundle.load(&mut routes).expect("routes");
        assert_eq!(routes.len(), 2);

        let mut app = App::new(Environment::Dev);
        app.register_bundle(DemoBundle).expect("demo");
        app.register_bundle(FrameworkBundle).expect("framework");
        app.boot().expect("boot");
        app.shutdown().expect("shutdown");
    }

    #[test]
    fn demo_hello_command_prints_greeting() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let packages = root.join("config/packages");
        let (_config, container) =
            demo_container(Some(packages.as_path()), "dev").expect("container");
        let container = Arc::new(container);
        let console = container
            .get_as::<Application>(CONSOLE_APPLICATION_SERVICE)
            .expect("console");
        console
            .run_with(["console", "demo:hello"], Some(Arc::clone(&container)))
            .expect("demo:hello");
    }

    #[test]
    fn demo_hello_without_container_is_ok() {
        let cmd = HelloCommand;
        let input = Input::new(Environment::Dev, true, Vec::new(), None);
        cmd.execute(&input).expect("no container");
    }

    #[test]
    fn demo_slug_command_uses_serenade_string() {
        let cmd = SlugCommand;
        let input = Input::new(Environment::Dev, true, vec!["Hello World!".into()], None);
        cmd.execute(&input).expect("slug");
        assert_eq!(slug("Hello World!"), "hello-world");
    }

    #[test]
    fn demo_workflow_publishes_draft() {
        let place = run_article_publish("article-test").expect("publish");
        assert_eq!(place, "published");
        let cmd = WorkflowCommand;
        let input = Input::new(Environment::Dev, true, Vec::new(), None);
        cmd.execute(&input).expect("workflow cmd");
    }
}
