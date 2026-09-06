//! Console Application: Symfony `bin/console` analogue.
//!
//! Plain commands use clap for argv (`--env`, `--no-debug`). Interactive mode
//! (`--interactive`) uses rustyline for ↑/↓ history. Debug surfaces may use
//! ratatui (see [`DebugContainerCommand`], [`DebugConfigCommand`]).

mod application;
mod command;
mod commands;
mod compile_pass;
mod error;
mod input;
mod interactive;

pub use application::{Application, stdout_is_terminal};
pub use command::Command;
pub use commands::{AboutCommand, DebugConfigCommand, DebugContainerCommand};
pub use compile_pass::{APPLICATION_SERVICE, COMMAND_TAG, CommandService, RegisterCommandsPass};
pub use error::ConsoleError;
pub use input::Input;
pub use interactive::{HISTORY_FILE_NAME, history_path};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
