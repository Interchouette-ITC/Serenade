//! Bundle registration contract used during kernel compile and boot.

use crate::KernelError;

/// Composition unit registered on a [`Kernel`](crate::Kernel).
///
/// Bundles run `build` during compile, then `boot` and `shutdown` in registration order.
/// Default implementations are no-ops so an empty bundle is valid.
pub trait Bundle: Send + Sync {
    /// Stable unique name used for duplicate detection and error reports.
    fn name(&self) -> &'static str;

    /// Compile-time wiring (container extensions land with the DI crate).
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
