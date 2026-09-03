//! Sample Serenade application: `FrameworkBundle` + `DemoBundle`.

use std::path::PathBuf;

use serenade_bundle::{
    build_container, BundleError, Extension, FrameworkBundle, FrameworkExtension, FRAMEWORK_BUNDLE,
    ROUTER_SERVICE,
};
use serenade_config::Config;
use serenade_di::{ContainerBuilder, ServiceDefinition};
use serenade_event::DISPATCHER_SERVICE;
use serenade_http::{Method, Route, RouteCollection, RouteLoader};
use serenade_kernel::{App, Application, BundleInterface, Environment};

struct DemoBundle;

impl BundleInterface for DemoBundle {
    fn name(&self) -> &'static str {
        "demo"
    }

    fn dependencies(&self) -> &'static [&'static str] {
        &[FRAMEWORK_BUNDLE]
    }
}

impl RouteLoader for DemoBundle {
    fn load(&self, collection: &mut RouteCollection) -> Result<(), serenade_http::HttpError> {
        collection.add(Route::with_method("healthz", "/healthz", Method::Get))
    }
}

struct DemoExtension;

impl Extension for DemoExtension {
    fn alias(&self) -> &'static str {
        "demo"
    }

    fn load(&self, config: &Config, builder: &mut ContainerBuilder) -> Result<(), BundleError> {
        config.apply_to(builder.parameters_mut());
        builder.register(ServiceDefinition::new("demo.greeting"), |container| {
            let name = container
                .parameters()
                .get("name")
                .map_or_else(|_| "world".to_owned(), str::to_owned);
            Ok(Box::new(format!("hello {name}")))
        })?;
        Ok(())
    }
}

fn main() -> Result<(), BundleError> {
    let mut app = App::new(Environment::Dev);
    app.register_bundle(DemoBundle)?;
    app.register_bundle(FrameworkBundle)?;
    app.boot()?;

    let packages = packages_dir();
    let (_config, container) = build_container(
        Some(packages.as_path()),
        &[&FrameworkExtension, &DemoExtension],
    )?;

    let greeting = container.get_as::<String>("demo.greeting")?;
    let dispatcher = container.get_as::<serenade_event::EventDispatcher>(DISPATCHER_SERVICE)?;
    let shared_router = container.get_as::<RouteCollection>(ROUTER_SERVICE)?;

    let mut collection = (*shared_router).clone();
    DemoBundle
        .load(&mut collection)
        .map_err(|error| BundleError::Extension {
            alias: "demo",
            message: error.to_string(),
        })?;

    println!("bundles: {:?}", app.kernel().bundle_names());
    println!("greeting: {greeting}");
    println!("event_dispatcher subscribers: {}", dispatcher.len());
    println!("routes: {}", collection.len());
    for route in collection.routes() {
        println!("  {} {} {:?}", route.name(), route.path(), route.methods());
    }

    app.shutdown()?;
    Ok(())
}

fn packages_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config/packages")
}
