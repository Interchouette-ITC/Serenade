//! Console Application runner (clap argv + registered commands).

use std::io::{self, IsTerminal, Write};
use std::sync::Arc;

use clap::{Arg, ArgAction, Command as ClapCommand};
use serenade_di::Container;
use serenade_kernel::Environment;

use crate::{Command, ConsoleError, Input};

/// Discoverable command registry and argv runner.
#[derive(Clone, Default)]
pub struct Application {
    commands: Vec<Arc<dyn Command>>,
}

impl Application {
    /// Empty application with no commands.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a command.
    pub fn add(&mut self, command: Arc<dyn Command>) {
        self.commands.push(command);
    }

    /// Registered commands in registration order.
    #[must_use]
    pub fn commands(&self) -> &[Arc<dyn Command>] {
        &self.commands
    }

    /// Looks up a command by name.
    #[must_use]
    pub fn find(&self, name: &str) -> Option<&Arc<dyn Command>> {
        self.commands.iter().find(|command| command.name() == name)
    }

    /// Parses `argv` (program name first) and runs the selected command.
    ///
    /// Global flags: `--env` / `APP_ENV`, `--no-debug`, `--interactive`.
    /// With no command or `list`, prints available commands. With
    /// `--interactive`, enters a rustyline REPL (↑/↓ history).
    ///
    /// # Errors
    ///
    /// Returns [`ConsoleError`] on parse failure, unknown command, or command error.
    pub fn run<I, S>(&self, argv: I) -> Result<(), ConsoleError>
    where
        I: IntoIterator<Item = S>,
        S: Into<std::ffi::OsString> + Clone,
    {
        self.run_with(argv, None)
    }

    /// Like [`Self::run`], attaching an optional DI [`Container`] for commands.
    ///
    /// # Errors
    ///
    /// Returns [`ConsoleError`] on parse failure, unknown command, or command error.
    pub fn run_with<I, S>(
        &self,
        argv: I,
        container: Option<Arc<Container>>,
    ) -> Result<(), ConsoleError>
    where
        I: IntoIterator<Item = S>,
        S: Into<std::ffi::OsString> + Clone,
    {
        let matches = clap_command()
            .try_get_matches_from(argv)
            .map_err(|error| ConsoleError::Failed(error.to_string()))?;

        let env_name = matches
            .get_one::<String>("env")
            .map_or("dev", String::as_str);
        let environment = Environment::from_name(env_name)
            .map_err(|error| ConsoleError::InvalidEnvironment(error.to_string()))?;
        let no_debug = matches.get_flag("no-debug");
        let debug = if no_debug {
            false
        } else {
            environment.is_debug()
        };

        if matches.get_flag("interactive") {
            return self.run_interactive(&environment, debug, container.as_ref());
        }

        let command_name = matches.get_one::<String>("command").map(String::as_str);
        let trailing: Vec<String> = matches
            .get_many::<String>("args")
            .map(|values| values.cloned().collect())
            .unwrap_or_default();

        match command_name {
            None | Some("list") => {
                self.print_list()?;
                Ok(())
            }
            Some(name) => {
                let Some(command) = self.find(name) else {
                    return Err(ConsoleError::NotFound(name.to_owned()));
                };
                let input = Input::new(environment, debug, trailing, container);
                command.execute(&input)
            }
        }
    }

    pub(crate) fn print_list(&self) -> Result<(), ConsoleError> {
        let mut out = io::stdout().lock();
        writeln!(out, "Serenade Console")?;
        writeln!(out)?;
        writeln!(out, "Available commands:")?;
        let mut names: Vec<_> = self.commands.iter().collect();
        names.sort_by(|left, right| left.name().cmp(right.name()));
        for command in names {
            writeln!(out, "  {:24} {}", command.name(), command.description())?;
        }
        writeln!(out)?;
        writeln!(
            out,
            "Usage: <binary> [--env ENV] [--no-debug] [--interactive] <command> [args…]"
        )?;
        Ok(())
    }
}

fn clap_command() -> ClapCommand {
    ClapCommand::new("serenade")
        .about("Serenade console (bin/console analogue)")
        .disable_help_subcommand(true)
        .arg(
            Arg::new("env")
                .long("env")
                .value_name("ENV")
                .default_value("dev")
                .env("APP_ENV")
                .help("Runtime environment (dev, test, prod, or custom)"),
        )
        .arg(
            Arg::new("no-debug")
                .long("no-debug")
                .action(ArgAction::SetTrue)
                .help("Disable debug even in dev/test"),
        )
        .arg(
            Arg::new("interactive")
                .long("interactive")
                .short('i')
                .action(ArgAction::SetTrue)
                .help("Interactive REPL with ↑/↓ command history"),
        )
        .arg(
            Arg::new("command")
                .value_name("COMMAND")
                .help("Command name (for example serenade:about)"),
        )
        .arg(
            Arg::new("args")
                .value_name("ARGS")
                .num_args(0..)
                .trailing_var_arg(true)
                .allow_hyphen_values(true)
                .help("Arguments passed to the command"),
        )
}

/// Returns whether stdout is an interactive terminal (for TUI commands).
#[must_use]
pub fn stdout_is_terminal() -> bool {
    io::stdout().is_terminal()
}
