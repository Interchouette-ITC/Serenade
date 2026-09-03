//! Assemble a DI container from package config and extensions.

use std::path::Path;

use serenade_config::{load_packages, Config};
use serenade_di::{Container, ContainerBuilder, ServiceDefinition};
use serenade_event::RegisterEventSubscribersPass;

use crate::framework::CONFIG_SERVICE;
use crate::{BundleError, Extension};

/// Loads `config/packages`, runs `extensions`, and compiles the container.
///
/// Flattened package parameters are applied first. The merged root config is
/// registered as [`CONFIG_SERVICE`]. Each extension then receives
/// `config.section(extension.alias())`. [`RegisterEventSubscribersPass`] always
/// runs so tagged subscribers populate `event_dispatcher`.
///
/// # Errors
///
/// Returns [`BundleError`] on config, extension, or DI failures.
pub fn build_container(
    packages_dir: Option<&Path>,
    extensions: &[&dyn Extension],
) -> Result<(Config, Container), BundleError> {
    let config = match packages_dir {
        Some(dir) => load_packages(dir)?.interpolate_env()?,
        None => Config::empty(),
    };
    let mut builder = ContainerBuilder::new();
    config.apply_to(builder.parameters_mut());
    let snapshot = config.clone();
    builder.register(ServiceDefinition::new(CONFIG_SERVICE), move |_| {
        Ok(Box::new(snapshot.clone()))
    })?;
    builder.add_compile_pass(RegisterEventSubscribersPass);
    for extension in extensions {
        let section = config.section(extension.alias());
        extension.load(&section, &mut builder).map_err(|error| {
            if matches!(error, BundleError::Extension { .. }) {
                error
            } else {
                BundleError::Extension {
                    alias: extension.alias(),
                    message: error.to_string(),
                }
            }
        })?;
    }
    Ok((config, builder.compile()?))
}
