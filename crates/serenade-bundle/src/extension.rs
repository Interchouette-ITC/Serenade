//! Dependency-injection extension contract (Symfony `Extension` analogue).

use serenade_config::Config;
use serenade_di::ContainerBuilder;

use crate::BundleError;

/// Loads package config for one alias into a [`ContainerBuilder`].
///
/// Bundles expose an extension; [`crate::build_container`] calls each in order.
pub trait Extension: Send + Sync {
    /// Config key and diagnostic name (for example `framework`).
    fn alias(&self) -> &'static str;

    /// Registers services and parameters from `config` (already scoped to this alias).
    ///
    /// # Errors
    ///
    /// Returns [`BundleError`] when registration fails.
    fn load(&self, config: &Config, builder: &mut ContainerBuilder) -> Result<(), BundleError>;
}
