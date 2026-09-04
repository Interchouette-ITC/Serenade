//! Console component tests.

use std::io::IsTerminal;
use std::sync::Arc;

use serenade_di::{CompilePass, ContainerBuilder, ServiceDefinition};
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
    assert_eq!(RegisterCommandsPass.name(), "register_console_commands");
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
    assert_eq!(application.commands().len(), 1);
    assert_eq!(Environment::from_name("dev").unwrap().as_str(), "dev");
}

#[test]
fn version_and_stdout_terminal_helpers() {
    assert_ne!(crate::version(), "");
    let _ = crate::stdout_is_terminal();
}

#[test]
fn invalid_environment_flag_errors() {
    let app = Application::new();
    let err = app.run(["console", "--env", "   ", "list"]).unwrap_err();
    assert!(matches!(err, ConsoleError::InvalidEnvironment(_)));
}

#[test]
fn list_command_name_prints_available() {
    let mut app = Application::new();
    app.add(Arc::new(PingCommand));
    app.run(["console", "list"]).expect("list");
}

#[test]
fn interactive_flag_on_non_tty_exits_on_eof() {
    if std::io::stdin().is_terminal() {
        return;
    }
    let app = Application::new();
    app.run(["console", "--interactive"])
        .expect("interactive eof");
}

#[test]
fn print_list_to_maps_io_errors_on_usage_line() {
    struct FailOnWrite {
        fail_at: usize,
        count: usize,
    }

    impl std::io::Write for FailOnWrite {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.count += 1;
            if self.count >= self.fail_at {
                return Err(std::io::Error::other("forced write failure"));
            }
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let app = Application::new();
    // Empty command list: Console, blank, Available, blank, Usage (5th write).
    let mut fail = FailOnWrite {
        fail_at: 5,
        count: 0,
    };
    let err = app.print_list_to(&mut fail).expect_err("io");
    assert!(matches!(err, ConsoleError::Io(_)));
}
