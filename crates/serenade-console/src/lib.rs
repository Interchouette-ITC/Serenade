//! Console Application: Symfony `bin/console` analogue.
//!
//! Plain commands use clap for argv (`--env`, `--no-debug`). Interactive debug
//! surfaces may use ratatui (see [`DebugContainerCommand`]).

mod application;
mod command;
mod commands;
mod compile_pass;
mod error;
mod input;

pub use application::{stdout_is_terminal, Application};
pub use command::Command;
pub use commands::{AboutCommand, DebugContainerCommand};
pub use compile_pass::{CommandService, RegisterCommandsPass, APPLICATION_SERVICE, COMMAND_TAG};
pub use error::ConsoleError;
pub use input::Input;

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
