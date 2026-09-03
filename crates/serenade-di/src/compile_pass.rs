//! Extensible compile-pass pipeline.

use crate::{ContainerBuilder, DiError};

/// Mutates a [`ContainerBuilder`] before it freezes into a container.
pub trait CompilePass: Send + Sync {
    /// Stable name for diagnostics.
    fn name(&self) -> &'static str;

    /// Applies this pass.
    ///
    /// # Errors
    ///
    /// Return [`DiError`] when the builder is invalid for this pass.
    fn process(&self, builder: &mut ContainerBuilder) -> Result<(), DiError>;
}
