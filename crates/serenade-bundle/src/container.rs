//! Assemble a DI container from package config and extensions.

use std::path::Path;

use serenade_config::{load_packages_for_env, Config};
use serenade_console::RegisterCommandsPass;
use serenade_di::{Container, ContainerBuilder, ServiceDefinition};
use serenade_event::RegisterEventSubscribersPass;

use crate::framework::CONFIG_SERVICE;
use crate::{BundleError, Extension};

/// Loads `config/packages` (plus `{environment}/` overlay), runs `extensions`, and compiles.
///
/// Flattened package parameters are applied first. The merged root config is
/// registered as [`CONFIG_SERVICE`]. Each extension then receives
/// `config.section(extension.alias())`. Event and console compile passes always
/// run so tagged subscribers and commands populate their services.
///
/// Call [`serenade_config::load_dotenv`] on the project root before this when
/// apps use `.env` files.
///
/// # Errors
///
/// Returns [`BundleError`] on config, extension, or DI failures.
pub fn build_container(
    packages_dir: Option<&Path>,
    environment: &str,
    extensions: &[&dyn Extension],
) -> Result<(Config, Container), BundleError> {
    let config = match packages_dir {
        Some(dir) => load_packages_for_env(dir, environment)?.interpolate_env()?,
        None => Config::empty(),
    };
    let mut builder = ContainerBuilder::new();
    config.apply_to(builder.parameters_mut());
    let snapshot = config.clone();
    builder.register(ServiceDefinition::new(CONFIG_SERVICE), move |_| {
        Ok(Box::new(snapshot.clone()))
    })?;
    builder.add_compile_pass(RegisterEventSubscribersPass);
    builder.add_compile_pass(RegisterCommandsPass);
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
