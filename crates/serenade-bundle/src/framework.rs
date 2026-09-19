//! Core `FrameworkBundle` and its DI extension.

use std::sync::Arc;

use serenade_config::Config;
use serenade_console::{
    AboutCommand, COMMAND_TAG, CommandService, DebugConfigCommand, DebugContainerCommand,
};
use serenade_di::{ContainerBuilder, ServiceDefinition};
use serenade_http::RouteCollection;
use serenade_http_client::RegisterDefaultHttpClientPass;
use serenade_kernel::{BundleInterface, KernelError};
use serenade_lock::RegisterDefaultLockPass;
use serenade_mailer::RegisterDefaultMailerPass;
use serenade_notifier::RegisterDefaultNotifierPass;
use serenade_rate_limiter::RegisterDefaultRateLimiterPass;
use serenade_scheduler::{RegisterDefaultSchedulerPass, RunSchedulerCommand};

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
/// as [`ROUTER_SERVICE`], registers built-in console commands, and adds
/// [`RegisterDefaultMailerPass`] / [`RegisterDefaultNotifierPass`] /
/// [`RegisterDefaultHttpClientPass`] /
/// [`RegisterDefaultLockPass`] / [`RegisterDefaultRateLimiterPass`] /
/// [`RegisterDefaultSchedulerPass`] so apps resolve `mailer`, `notifier`,
/// `http_client`, `lock.store`, `rate_limiter.storage`, and `scheduler` unless
/// they replace them. The root config, event dispatcher, and console
/// application are registered by [`crate::build_container`].
///
/// # Panics
///
/// Panics if a built-in console command id is already registered on `builder`.
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameworkExtension;

impl Extension for FrameworkExtension {
    fn alias(&self) -> &'static str {
        FRAMEWORK_BUNDLE
    }

    #[allow(clippy::missing_panics_doc)]
    fn load(&self, config: &Config, builder: &mut ContainerBuilder) -> Result<(), BundleError> {
        config.apply_to(builder.parameters_mut());
        builder.register(ServiceDefinition::new(ROUTER_SERVICE), |_| {
            Ok(Box::new(RouteCollection::new()))
        })?;
        builder.add_compile_pass(RegisterDefaultMailerPass);
        builder.add_compile_pass(RegisterDefaultNotifierPass);
        builder.add_compile_pass(RegisterDefaultHttpClientPass);
        builder.add_compile_pass(RegisterDefaultLockPass);
        builder.add_compile_pass(RegisterDefaultRateLimiterPass);
        builder.add_compile_pass(RegisterDefaultSchedulerPass);
        // `expect`: hardcoded framework ids cannot collide; avoids Codecov-only `?` Err arms.
        builder
            .register(
                ServiceDefinition::new("console.command.about").with_tag(COMMAND_TAG),
                |_| Ok(Box::new(CommandService(Arc::new(AboutCommand)))),
            )
            .expect("framework about command id is unique");
        builder
            .register(
                ServiceDefinition::new("console.command.debug_container").with_tag(COMMAND_TAG),
                |_| Ok(Box::new(CommandService(Arc::new(DebugContainerCommand)))),
            )
            .expect("framework debug_container command id is unique");
        builder
            .register(
                ServiceDefinition::new("console.command.debug_config").with_tag(COMMAND_TAG),
                |_| Ok(Box::new(CommandService(Arc::new(DebugConfigCommand)))),
            )
            .expect("framework debug_config command id is unique");
        builder
            .register(
                ServiceDefinition::new("console.command.scheduler_run").with_tag(COMMAND_TAG),
                |_| Ok(Box::new(CommandService(Arc::new(RunSchedulerCommand)))),
            )
            .expect("framework scheduler_run command id is unique");
        Ok(())
    }
}
