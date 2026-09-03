//! Console component tests.

use std::sync::Arc;

use serenade_di::{ContainerBuilder, ServiceDefinition};
use serenade_kernel::Environment;

use crate::{
    AboutCommand, Application, Command, CommandService, ConsoleError, DebugConfigCommand,
    DebugContainerCommand, Input, RegisterCommandsPass, APPLICATION_SERVICE, COMMAND_TAG,
};

struct PingCommand;

impl Command for PingCommand {
    fn name(&self) -> &'static str {
        "app:ping"
    }

    fn description(&self) -> &'static str {
        "Responds with pong"
    }

    fn execute(&self, _input: &Input) -> Result<(), ConsoleError> {
        println!("pong");
        Ok(())
    }
}

#[test]
fn about_prints_without_error() {
    let mut app = Application::new();
    app.add(Arc::new(AboutCommand));
    app.run(["console", "serenade:about"]).expect("about");
}

#[test]
fn list_when_no_command() {
    let mut app = Application::new();
    app.add(Arc::new(PingCommand));
    app.run(["console"]).expect("list");
}

#[test]
fn env_and_no_debug_flags() {
    let mut app = Application::new();
    app.add(Arc::new(AboutCommand));
    app.run(["console", "--env", "prod", "--no-debug", "serenade:about"])
        .expect("about prod");
}

#[test]
fn unknown_command_errors() {
    let app = Application::new();
    let error = app.run(["console", "missing:cmd"]).unwrap_err();
    assert!(matches!(error, ConsoleError::NotFound(_)));
}

#[test]
fn custom_env_parses() {
    let mut app = Application::new();
    app.add(Arc::new(AboutCommand));
    app.run(["console", "--env", "staging", "serenade:about"])
        .expect("staging");
}

#[test]
fn debug_container_plain() {
    let mut builder = ContainerBuilder::new();
    builder
        .register(ServiceDefinition::new("demo"), |_| {
            Ok(Box::new(String::from("x")))
        })
        .expect("register");
    let container = Arc::new(builder.compile().expect("compile"));
    let mut app = Application::new();
    app.add(Arc::new(DebugContainerCommand));
    app.run_with(["console", "debug:container", "--plain"], Some(container))
        .expect("debug");
}

#[test]
fn debug_config_requires_debug_and_redacts() {
    let mut builder = ContainerBuilder::new();
    builder.parameters_mut().set("framework.secret", "s3cret");
    builder.parameters_mut().set("demo.name", "shop");
    let container = Arc::new(builder.compile().expect("compile"));
    let mut app = Application::new();
    app.add(Arc::new(DebugConfigCommand));
    let err = app
        .run_with(
            ["console", "--no-debug", "debug:config", "--plain"],
            Some(Arc::clone(&container)),
        )
        .unwrap_err();
    assert!(matches!(err, ConsoleError::Failed(_)));
    app.run_with(
        ["console", "debug:config", "--plain"],
        Some(Arc::clone(&container)),
    )
    .expect("debug config");
}

#[test]
fn compile_pass_collects_tagged_commands() {
    let mut builder = ContainerBuilder::new();
    builder.add_compile_pass(RegisterCommandsPass);
    builder
        .register(
            ServiceDefinition::new("cmd.ping").with_tag(COMMAND_TAG),
            |_| Ok(Box::new(CommandService(Arc::new(PingCommand)))),
        )
        .expect("register");
    let container = builder.compile().expect("compile");
    let application = container
        .get_as::<Application>(APPLICATION_SERVICE)
        .expect("application");
    assert!(application.find("app:ping").is_some());
    assert_eq!(Environment::from_name("dev").unwrap().as_str(), "dev");
}
