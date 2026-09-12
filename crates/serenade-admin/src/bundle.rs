//! `AdminBundle` and DI extension.

use serenade_bundle::{BundleError, Extension, FRAMEWORK_BUNDLE};
use serenade_config::Config;
use serenade_di::{ContainerBuilder, ServiceDefinition};
use serenade_kernel::BundleInterface;

use crate::AdminRegistry;

/// Canonical bundle name.
pub const ADMIN_BUNDLE: &str = "admin";

/// DI service id for [`AdminRegistry`].
pub const ADMIN_REGISTRY_SERVICE: &str = "admin.registry";

/// Optional Admin CRUD bundle (not part of `FrameworkBundle`).
#[derive(Clone, Copy, Debug, Default)]
pub struct AdminBundle;

impl BundleInterface for AdminBundle {
    fn name(&self) -> &'static str {
        ADMIN_BUNDLE
    }

    fn dependencies(&self) -> &'static [&'static str] {
        &[FRAMEWORK_BUNDLE]
    }
}

/// DI extension for the `admin` package key.
///
/// Registers an empty [`AdminRegistry`] as [`ADMIN_REGISTRY_SERVICE`]. Apps add
/// resources in code (or a later config loader) before compiling routes.
#[derive(Clone, Copy, Debug, Default)]
pub struct AdminExtension;

impl Extension for AdminExtension {
    fn alias(&self) -> &'static str {
        ADMIN_BUNDLE
    }

    fn load(&self, config: &Config, builder: &mut ContainerBuilder) -> Result<(), BundleError> {
        config.apply_to(builder.parameters_mut());
        builder.register(ServiceDefinition::new(ADMIN_REGISTRY_SERVICE), |_| {
            Ok(Box::new(AdminRegistry::new()))
        })?;
        Ok(())
    }
}
