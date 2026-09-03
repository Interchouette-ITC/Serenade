//! Kernel state machine: register, compile, boot, shutdown.

use std::fmt::{Display, Formatter};

use crate::{Bundle, Environment, KernelError};

/// Lifecycle phase of a [`Kernel`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum KernelPhase {
    /// Bundles may still be registered.
    Created,
    /// `build` has run; [`Kernel::boot`] is allowed.
    Compiled,
    /// `boot` has run; [`Kernel::shutdown`] is allowed.
    Booted,
    /// Terminal phase after shutdown.
    Shutdown,
}

impl Display for KernelPhase {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Created => "created",
            Self::Compiled => "compiled",
            Self::Booted => "booted",
            Self::Shutdown => "shutdown",
        })
    }
}

/// Application kernel: environment, bundle order, and lifecycle.
pub struct Kernel {
    environment: Environment,
    debug: bool,
    bundles: Vec<Box<dyn Bundle>>,
    phase: KernelPhase,
}

impl Kernel {
    /// Creates a kernel in [`KernelPhase::Created`].
    ///
    /// Debug defaults to [`Environment::is_debug`].
    #[must_use]
    pub fn new(environment: Environment) -> Self {
        Self {
            environment,
            debug: environment.is_debug(),
            bundles: Vec::new(),
            phase: KernelPhase::Created,
        }
    }

    /// Overrides the debug flag. Call before [`Self::compile`].
    #[must_use]
    pub const fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    /// Runtime environment.
    #[must_use]
    pub const fn environment(&self) -> Environment {
        self.environment
    }

    /// Effective debug flag.
    #[must_use]
    pub const fn debug(&self) -> bool {
        self.debug
    }

    /// Current lifecycle phase.
    #[must_use]
    pub const fn phase(&self) -> KernelPhase {
        self.phase
    }

    /// Registered bundle names in order.
    #[must_use]
    pub fn bundle_names(&self) -> Vec<&'static str> {
        self.bundles.iter().map(|bundle| bundle.name()).collect()
    }

    /// Registers a bundle. Must run while the kernel is [`KernelPhase::Created`].
    ///
    /// # Errors
    ///
    /// Returns [`KernelError::InvalidState`] after compile, or
    /// [`KernelError::DuplicateBundle`] when `bundle.name()` is already registered.
    pub fn register_bundle(&mut self, bundle: impl Bundle + 'static) -> Result<(), KernelError> {
        self.ensure_phase("register", KernelPhase::Created)?;
        let name = bundle.name();
        if self.bundles.iter().any(|existing| existing.name() == name) {
            return Err(KernelError::DuplicateBundle(name));
        }
        self.bundles.push(Box::new(bundle));
        Ok(())
    }

    /// Runs [`Bundle::build`] on each bundle in registration order.
    ///
    /// # Errors
    ///
    /// Returns [`KernelError::InvalidState`] unless the kernel is [`KernelPhase::Created`].
    pub fn compile(&mut self) -> Result<(), KernelError> {
        self.ensure_phase("compile", KernelPhase::Created)?;
        for bundle in &self.bundles {
            bundle
                .build()
                .map_err(|error| wrap_bundle(bundle.name(), "build", error))?;
        }
        self.phase = KernelPhase::Compiled;
        Ok(())
    }

    /// Compiles if needed, then runs [`Bundle::boot`] in registration order.
    ///
    /// # Errors
    ///
    /// Returns [`KernelError::InvalidState`] when already booted or shut down.
    pub fn boot(&mut self) -> Result<(), KernelError> {
        if self.phase == KernelPhase::Created {
            self.compile()?;
        }
        self.ensure_phase("boot", KernelPhase::Compiled)?;
        for bundle in &self.bundles {
            bundle
                .boot()
                .map_err(|error| wrap_bundle(bundle.name(), "boot", error))?;
        }
        self.phase = KernelPhase::Booted;
        Ok(())
    }

    /// Runs [`Bundle::shutdown`] in reverse registration order.
    ///
    /// # Errors
    ///
    /// Returns [`KernelError::InvalidState`] unless the kernel is [`KernelPhase::Booted`].
    pub fn shutdown(&mut self) -> Result<(), KernelError> {
        self.ensure_phase("shutdown", KernelPhase::Booted)?;
        for bundle in self.bundles.iter().rev() {
            bundle
                .shutdown()
                .map_err(|error| wrap_bundle(bundle.name(), "shutdown", error))?;
        }
        self.phase = KernelPhase::Shutdown;
        Ok(())
    }

    fn ensure_phase(&self, action: &'static str, expected: KernelPhase) -> Result<(), KernelError> {
        if self.phase == expected {
            Ok(())
        } else {
            Err(KernelError::InvalidState {
                action,
                state: self.phase,
            })
        }
    }
}

impl std::fmt::Debug for Kernel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Kernel")
            .field("environment", &self.environment)
            .field("debug", &self.debug)
            .field("phase", &self.phase)
            .field("bundles", &self.bundle_names())
            .finish()
    }
}

fn wrap_bundle(bundle: &'static str, phase: &'static str, error: KernelError) -> KernelError {
    match error {
        KernelError::Bundle { .. } => error,
        other => KernelError::Bundle {
            bundle,
            phase,
            message: other.to_string(),
        },
    }
}
