//! Facade-level tests: the curated surface is reachable through `serenade_framework` only.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use serenade_framework::prelude::*;

#[test]
fn version_matches_package() {
    assert_eq!(serenade_framework::version(), env!("CARGO_PKG_VERSION"));
}

struct Catalog;

impl BundleInterface for Catalog {
    fn name(&self) -> &'static str {
        "catalog"
    }
}

struct Checkout;

impl BundleInterface for Checkout {
    fn name(&self) -> &'static str {
        "checkout"
    }

    fn dependencies(&self) -> &'static [&'static str] {
        &["catalog"]
    }
}

#[test]
fn kernel_lifecycle_through_prelude() {
    let mut app = App::new(Environment::Test);
    app.register_bundle(Checkout).expect("register checkout");
    app.register_bundle(Catalog).expect("register catalog");
    app.boot().expect("boot");
    assert_eq!(app.kernel().phase(), KernelPhase::Booted);
    assert_eq!(app.kernel().bundle_names(), vec!["catalog", "checkout"]);
    app.shutdown().expect("shutdown");
    assert_eq!(app.kernel().phase(), KernelPhase::Shutdown);
}

#[test]
fn unknown_bundle_dependency_is_a_kernel_error() {
    let mut app = App::new(Environment::Test);
    app.register_bundle(Checkout).expect("register");
    let error: serenade_framework::kernel::KernelError = app.boot().expect_err("missing dep");
    assert_ne!(error.to_string(), "");
}

#[test]
fn container_resolves_registered_service() {
    let mut builder = ContainerBuilder::new();
    builder
        .register(ServiceDefinition::new("greeting"), |_| {
            Ok(Box::new(String::from("hello")))
        })
        .expect("register");
    let container: Container = builder.compile().expect("compile");
    let greeting = container.get_as::<String>("greeting").expect("resolve");
    assert_eq!(greeting.as_str(), "hello");
}

struct OrderPlaced;

impl Event for OrderPlaced {
    fn name(&self) -> &'static str {
        "order.placed"
    }
}

struct CountOrders(Arc<AtomicUsize>);

impl EventSubscriber for CountOrders {
    fn event_name(&self) -> &'static str {
        "order.placed"
    }

    fn handle(&self, _event: &dyn Event) -> Result<(), serenade_framework::event::EventError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[test]
fn dispatcher_reaches_subscriber() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut dispatcher = EventDispatcher::new();
    dispatcher.add(Arc::new(CountOrders(Arc::clone(&count))));
    dispatcher.dispatch(&OrderPlaced).expect("dispatch");
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[test]
fn config_flattens_into_parameter_bag() {
    let config = Config::from_toml("[shop]\ncurrency = \"EUR\"\n").expect("toml");
    let mut bag = serenade_framework::di::ParameterBag::default();
    config.apply_to(&mut bag);
    assert_eq!(bag.get("shop.currency").expect("param"), "EUR");
}

#[cfg(feature = "http")]
#[test]
fn http_types_are_exposed() {
    use serenade_framework::http::{Method, Request, Response};

    let request = Request::new(Method::Get, "/health");
    assert_eq!(request.path(), "/health");
    assert_eq!(Response::text(200, "ok").status(), 200);
    let _client = serenade_framework::http::client::MockHttpClient::default();
}

#[cfg(feature = "bundle")]
#[test]
fn bundle_builds_without_network_transports() {
    use serenade_framework::bundle::{FrameworkExtension, ROUTER_SERVICE, build_container};

    let (_, container) = build_container(None, "test", &[&FrameworkExtension]).expect("container");
    assert!(container.definition(ROUTER_SERVICE).is_some());
}

#[cfg(all(feature = "bundle", feature = "http-client-reqwest"))]
#[test]
fn bundle_registers_explicit_reqwest_client() {
    use serenade_framework::bundle::{FrameworkExtension, build_container};
    use serenade_framework::http::client::{DEFAULT_HTTP_CLIENT_SERVICE, HttpClientService};

    let (_, container) = build_container(None, "test", &[&FrameworkExtension]).expect("container");
    container
        .get_as::<HttpClientService>(DEFAULT_HTTP_CLIENT_SERVICE)
        .expect("explicit reqwest client");
}

#[cfg(feature = "mailer-smtp")]
#[test]
fn smtp_transport_is_explicitly_available() {
    serenade_framework::mailer::SmtpTransport::unencrypted_localhost()
        .build()
        .expect("explicit SMTP transport");
}

#[cfg(feature = "validation")]
#[test]
fn validator_constraints_are_exposed() {
    use serenade_framework::validator::{Constraint, ConstraintViolationList, NotBlank};

    let mut violations = ConstraintViolationList::new();
    NotBlank.validate("  ", "name", &mut violations);
    assert_eq!(violations.len(), 1);
}

#[cfg(feature = "mailer")]
#[test]
fn mailer_without_backend_has_null_transport() {
    let transport = serenade_framework::mailer::NullTransport;
    assert_eq!(format!("{transport:?}"), "NullTransport");
}

#[cfg(feature = "form")]
#[test]
fn form_and_view_share_escaping() {
    let raw = "<b>\"x\"</b>";
    assert_eq!(
        serenade_framework::form::escape_html(raw),
        serenade_framework::view::escape_html(raw)
    );
}
