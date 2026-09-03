//! `serenade:about` - plain framework banner.

use crate::{Command, ConsoleError, Input};

/// Prints Serenade version, environment, and debug flag.
#[derive(Clone, Copy, Debug, Default)]
pub struct AboutCommand;

impl Command for AboutCommand {
    fn name(&self) -> &'static str {
        "serenade:about"
    }

    fn description(&self) -> &'static str {
        "Display Serenade version and runtime environment"
    }

    fn execute(&self, input: &Input) -> Result<(), ConsoleError> {
        println!("Serenade Console");
        println!("  kernel:      {}", serenade_kernel::version());
        println!("  console:     {}", env!("CARGO_PKG_VERSION"));
        println!("  environment: {}", input.environment());
        println!("  debug:       {}", input.debug());
        Ok(())
    }
}
