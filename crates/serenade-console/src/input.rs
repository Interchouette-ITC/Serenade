//! Parsed console input.

use std::sync::Arc;

use serenade_di::Container;
use serenade_kernel::Environment;

/// Context and trailing arguments for a command run.
#[derive(Clone)]
pub struct Input {
    environment: Environment,
    debug: bool,
    args: Vec<String>,
    container: Option<Arc<Container>>,
}

impl Input {
    /// Builds input for a command invocation.
    #[must_use]
    pub const fn new(
        environment: Environment,
        debug: bool,
        args: Vec<String>,
        container: Option<Arc<Container>>,
    ) -> Self {
        Self {
            environment,
            debug,
            args,
            container,
        }
    }

    /// Active environment (`--env` / `APP_ENV`).
    #[must_use]
    pub const fn environment(&self) -> &Environment {
        &self.environment
    }

    /// Whether debug is enabled (false when `--no-debug`).
    #[must_use]
    pub const fn debug(&self) -> bool {
        self.debug
    }

    /// Trailing arguments after the command name.
    #[must_use]
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// Optional DI container for kernel-aware commands.
    #[must_use]
    pub const fn container(&self) -> Option<&Arc<Container>> {
        self.container.as_ref()
    }
}
