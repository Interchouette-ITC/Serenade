//! Command trait.

use crate::{ConsoleError, Input};

/// A discoverable console command (`foo:bar`).
pub trait Command: Send + Sync {
    /// Stable command name (for example `serenade:about`).
    fn name(&self) -> &'static str;

    /// One-line help shown by `list`.
    fn description(&self) -> &'static str;

    /// Runs the command.
    ///
    /// # Errors
    ///
    /// Returns [`ConsoleError`] when the command fails.
    fn execute(&self, input: &Input) -> Result<(), ConsoleError>;
}
