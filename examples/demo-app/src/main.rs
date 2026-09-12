//! Sample Serenade application: `FrameworkBundle` + `DemoBundle`.

use std::path::PathBuf;

use serenade_bundle::{BundleError, FRAMEWORK_BUNDLE, FrameworkBundle, ROUTER_SERVICE};
use serenade_event::DISPATCHER_SERVICE;
use serenade_http::{RouteCollection, RouteLoader};
use serenade_kernel::{App, Application, Environment};
use serenade_observability::{APP, KERNEL, LoggingConfig, init};

use serenade_demo_app::{DEMO_GREETING_SERVICE, DemoBundle, DemoReady, demo_container};

fn main() -> Result<(), BundleError> {
    let env_name = std::env::var("APP_ENV").unwrap_or_else(|_| "dev".to_owned());
    let environment =
        Environment::from_name(&env_name).map_err(|error| BundleError::Extension {
            alias: "demo",
            message: error.to_string(),
        })?;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    serenade_config::load_dotenv(&root, environment.as_str()).map_err(|error| {
        BundleError::Extension {
            alias: "demo",
            message: error.to_string(),
        }
    })?;

    let log_dir = root.join("var/log");
    let _logging =
        init(&LoggingConfig::for_environment(&environment, &log_dir)).map_err(|error| {
            BundleError::Extension {
                alias: "demo",
                message: error.to_string(),
            }
        })?;
    tracing::info!(target: KERNEL, environment = environment.as_str(), "demo boot");

    let mut app = App::new(environment.clone());
    app.register_bundle(DemoBundle)?;
    app.register_bundle(FrameworkBundle)?;
    app.boot()?;

    let packages = root.join("config/packages");
    let (_config, container) = demo_container(Some(packages.as_path()), environment.as_str())?;

    let greeting = container.get_as::<String>(DEMO_GREETING_SERVICE)?;
    let dispatcher = container.get_as::<serenade_event::EventDispatcher>(DISPATCHER_SERVICE)?;
    let shared_router = container.get_as::<RouteCollection>(ROUTER_SERVICE)?;

    let mut collection = (*shared_router).clone();
    DemoBundle
        .load(&mut collection)
        .map_err(|error| BundleError::Extension {
            alias: "demo",
            message: error.to_string(),
        })?;

    dispatcher
        .dispatch(&DemoReady)
        .map_err(|error| BundleError::Extension {
            alias: "demo",
            message: error.to_string(),
        })?;

    println!("bundles: {:?}", app.kernel().bundle_names());
    println!("depends on: {FRAMEWORK_BUNDLE}");
    println!("greeting: {greeting}");
    println!("event_dispatcher subscribers: {}", dispatcher.len());
    println!("routes: {}", collection.len());
    for route in collection.routes() {
        println!("  {} {} {:?}", route.name(), route.path(), route.methods());
    }
    tracing::info!(
        target: APP,
        bundles = app.kernel().bundle_names().len(),
        routes = collection.len(),
        "demo ready"
    );

    app.shutdown()?;
    Ok(())
}
