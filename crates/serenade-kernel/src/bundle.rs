//! Bundle registration contract used during kernel compile and boot.

use crate::KernelError;

/// Composition unit registered on a [`Kernel`](crate::Kernel).
///
/// Bundles declare [`Self::dependencies`]; the kernel topologically sorts them
/// before `build` / `boot` so dependents run after their dependencies.
/// Default implementations are no-ops so an empty bundle is valid.
///
/// `BundleInterface` is the Symfony-shaped name; [`Bundle`] is the same trait.
pub trait BundleInterface: Send + Sync {
    /// Stable unique name used for duplicate detection and error reports.
    fn name(&self) -> &'static str;

    /// Bundle names that must compile and boot before this one.
    ///
    /// Empty by default. Unknown names and cycles fail at [`crate::Kernel::compile`].
    fn dependencies(&self) -> &'static [&'static str] {
        &[]
    }

    /// Compile-time wiring (register services on the DI container).
    ///
    /// # Errors
    ///
    /// Return [`KernelError`] when the bundle cannot be compiled.
    fn build(&self) -> Result<(), KernelError> {
        Ok(())
    }

    /// Runtime warmup after the kernel has compiled.
    ///
    /// # Errors
    ///
    /// Return [`KernelError`] when the bundle cannot start.
    fn boot(&self) -> Result<(), KernelError> {
        Ok(())
    }

    /// Ordered teardown after [`Kernel::shutdown`](crate::Kernel::shutdown).
    ///
    /// # Errors
    ///
    /// Return [`KernelError`] when teardown fails.
    fn shutdown(&self) -> Result<(), KernelError> {
        Ok(())
    }
}

/// Alias for [`BundleInterface`] (Symfony `Bundle` naming habit).
pub use BundleInterface as Bundle;
