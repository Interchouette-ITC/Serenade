//! Default application wrapper around [`Kernel`].

use crate::{Bundle, Environment, Kernel, KernelError};

/// Host that owns a [`Kernel`].
pub trait Application {
    /// Shared kernel access.
    fn kernel(&self) -> &Kernel;

    /// Boots the kernel (compile then boot).
    ///
    /// # Errors
    ///
    /// Propagates [`Kernel::boot`] errors.
    fn boot(&mut self) -> Result<(), KernelError>;

    /// Shuts the kernel down.
    ///
    /// # Errors
    ///
    /// Propagates [`Kernel::shutdown`] errors.
    fn shutdown(&mut self) -> Result<(), KernelError>;
}

/// Default application: a kernel plus ordered bundle registration.
pub struct App {
    kernel: Kernel,
}

impl App {
    /// Creates an application in [`crate::KernelPhase::Created`].
    #[must_use]
    pub fn new(environment: Environment) -> Self {
        Self {
            kernel: Kernel::new(environment),
        }
    }

    /// Overrides debug on the inner kernel. Call before [`Application::boot`].
    #[must_use]
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.kernel = self.kernel.with_debug(debug);
        self
    }

    /// Registers a bundle on the inner kernel.
    ///
    /// # Errors
    ///
    /// Propagates [`Kernel::register_bundle`] errors.
    pub fn register_bundle(&mut self, bundle: impl Bundle + 'static) -> Result<(), KernelError> {
        self.kernel.register_bundle(bundle)
    }
}

impl Application for App {
    fn kernel(&self) -> &Kernel {
        &self.kernel
    }

    fn boot(&mut self) -> Result<(), KernelError> {
        self.kernel.boot()
    }

    fn shutdown(&mut self) -> Result<(), KernelError> {
        self.kernel.shutdown()
    }
}
