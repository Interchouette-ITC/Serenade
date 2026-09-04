//! Booted application kernel for tests.

use serenade_kernel::{App, Application, BundleInterface, Environment, Kernel, KernelError};

/// Application wrapper that defaults to [`Environment::Test`].
///
/// # Examples
///
/// ```
/// use serenade_kernel::{Application, BundleInterface};
/// use serenade_testing::SerenadeTestKernel;
///
/// struct Demo;
///
/// impl BundleInterface for Demo {
///     fn name(&self) -> &'static str {
///         "demo"
///     }
/// }
///
/// let mut app = SerenadeTestKernel::new();
/// app.register_bundle(Demo).expect("register");
/// app.boot().expect("boot");
/// assert_eq!(app.kernel().bundle_names(), vec!["demo"]);
/// ```
pub struct SerenadeTestKernel {
    app: App,
}

impl SerenadeTestKernel {
    /// Creates an unbooted app in the test environment.
    #[must_use]
    pub fn new() -> Self {
        Self {
            app: App::new(Environment::Test),
        }
    }

    /// Registers a bundle before [`Self::boot`].
    ///
    /// # Errors
    ///
    /// Propagates [`App::register_bundle`](serenade_kernel::App::register_bundle) errors.
    pub fn register_bundle(
        &mut self,
        bundle: impl BundleInterface + 'static,
    ) -> Result<(), KernelError> {
        self.app.register_bundle(bundle)
    }

    /// Boots the inner application.
    ///
    /// # Errors
    ///
    /// Propagates [`Application::boot`] errors.
    pub fn boot(&mut self) -> Result<(), KernelError> {
        self.app.boot()
    }

    /// Shuts the inner application down.
    ///
    /// # Errors
    ///
    /// Propagates [`Application::shutdown`] errors.
    pub fn shutdown(&mut self) -> Result<(), KernelError> {
        self.app.shutdown()
    }

    /// Shared kernel access after construction.
    #[must_use]
    pub fn kernel(&self) -> &Kernel {
        self.app.kernel()
    }
}

impl Default for SerenadeTestKernel {
    fn default() -> Self {
        Self::new()
    }
}
