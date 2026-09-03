use std::path::PathBuf;
use std::sync::Arc;

use serenade_config::Config;
use serenade_di::{ContainerBuilder, ServiceDefinition};
use serenade_event::{EventDispatcher, DISPATCHER_SERVICE};
use serenade_http::RouteCollection;
use serenade_kernel::{App, Application, BundleInterface, BundleRegistry, Environment};

use super::{
    build_container, version, BundleError, Extension, FrameworkBundle, FrameworkExtension,
    CONFIG_SERVICE, FRAMEWORK_BUNDLE, ROUTER_SERVICE,
};

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

#[test]
fn version_is_non_empty() {
    assert_ne!(version(), "");
}

#[test]
fn registry_sorts_framework_before_app() {
    struct AppBundle;

    impl BundleInterface for AppBundle {
        fn name(&self) -> &'static str {
            "app"
        }

        fn dependencies(&self) -> &'static [&'static str] {
            &[FRAMEWORK_BUNDLE]
        }
    }

    let mut registry = BundleRegistry::new();
    registry.register(AppBundle).expect("app");
    registry.register(FrameworkBundle).expect("framework");
    let sorted = registry.sorted().expect("sort");
    let names: Vec<_> = sorted.iter().map(|bundle| bundle.name()).collect();
    assert_eq!(names, [FRAMEWORK_BUNDLE, "app"]);
}

#[test]
fn framework_extension_wires_router_config_and_dispatcher() {
    let dir = fixtures_packages();
    let (config, container) =
        build_container(Some(dir.as_path()), &[&FrameworkExtension]).expect("container");
    assert_eq!(
        config
            .parameters()
            .get("framework.secret")
            .map(String::as_str),
        Some("test")
    );
    let router = container
        .get_as::<RouteCollection>(ROUTER_SERVICE)
        .expect("router");
    assert!(router.is_empty());
    let stored = container.get_as::<Config>(CONFIG_SERVICE).expect("config");
    assert_eq!(
        stored
            .parameters()
            .get("framework.secret")
            .map(String::as_str),
        Some("test")
    );
    let dispatcher = container
        .get_as::<EventDispatcher>(DISPATCHER_SERVICE)
        .expect("dispatcher");
    assert!(dispatcher.is_empty());
    let console = container
        .get_as::<serenade_console::Application>(super::CONSOLE_APPLICATION_SERVICE)
        .expect("console");
    assert!(console.find("serenade:about").is_some());
    assert!(console.find("debug:container").is_some());
    assert!(Arc::strong_count(&router) >= 1);
}

#[test]
fn demo_extension_reads_package_section() {
    let dir = fixtures_packages();
    let (_config, container) =
        build_container(Some(dir.as_path()), &[&FrameworkExtension, &DemoExtension])
            .expect("container");
    let greeting = container
        .get_as::<String>("demo.greeting")
        .expect("greeting");
    assert_eq!(greeting.as_str(), "hello serenade");
}

#[test]
fn framework_bundle_boots_on_kernel() {
    let mut app = App::new(Environment::Test);
    app.register_bundle(FrameworkBundle).expect("register");
    app.boot().expect("boot");
    assert_eq!(app.kernel().bundle_names(), [FRAMEWORK_BUNDLE]);
    app.shutdown().expect("shutdown");
}

fn fixtures_packages() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/packages")
}
