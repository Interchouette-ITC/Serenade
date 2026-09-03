//! Core `FrameworkBundle` and its DI extension.

use std::sync::Arc;

use serenade_config::Config;
use serenade_console::{AboutCommand, CommandService, DebugContainerCommand, COMMAND_TAG};
use serenade_di::{ContainerBuilder, ServiceDefinition};
use serenade_http::RouteCollection;
use serenade_kernel::{BundleInterface, KernelError};

use crate::{BundleError, Extension};

/// Canonical name for [`FrameworkBundle`].
pub const FRAMEWORK_BUNDLE: &str = "framework";

/// Service id for the shared [`RouteCollection`].
pub const ROUTER_SERVICE: &str = "router";

/// Service id for the merged root [`Config`] snapshot.
pub const CONFIG_SERVICE: &str = "config";

/// Core Serenade bundle: wires router and participates in container compile.
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameworkBundle;

impl BundleInterface for FrameworkBundle {
    fn name(&self) -> &'static str {
        FRAMEWORK_BUNDLE
    }

    fn build(&self) -> Result<(), KernelError> {
        Ok(())
    }
}

/// DI extension for the `framework` package key.
///
/// Applies framework package parameters, registers an empty [`RouteCollection`]
/// as [`ROUTER_SERVICE`], and registers built-in console commands. The root
/// config, event dispatcher, and console application are registered by
/// [`crate::build_container`].
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameworkExtension;

impl Extension for FrameworkExtension {
    fn alias(&self) -> &'static str {
        FRAMEWORK_BUNDLE
    }

    fn load(&self, config: &Config, builder: &mut ContainerBuilder) -> Result<(), BundleError> {
        config.apply_to(builder.parameters_mut());
        builder.register(ServiceDefinition::new(ROUTER_SERVICE), |_| {
            Ok(Box::new(RouteCollection::new()))
        })?;
        builder.register(
            ServiceDefinition::new("console.command.about").with_tag(COMMAND_TAG),
            |_| Ok(Box::new(CommandService(Arc::new(AboutCommand)))),
        )?;
        builder.register(
            ServiceDefinition::new("console.command.debug_container").with_tag(COMMAND_TAG),
            |_| Ok(Box::new(CommandService(Arc::new(DebugContainerCommand)))),
        )?;
        Ok(())
    }
}
