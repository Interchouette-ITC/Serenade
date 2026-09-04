use std::sync::{Arc, Mutex};

use serenade_event::{Event, EventDispatcher};
use serenade_http::{HttpKernel, Method, Request, Response};
use serenade_kernel::{BundleInterface, KernelPhase};

use super::{assert_dispatched, version, HttpTestClient, RecordingSubscriber, SerenadeTestKernel};

struct EmptyBundle;

impl BundleInterface for EmptyBundle {
    fn name(&self) -> &'static str {
        "empty"
    }
}

#[derive(Clone, Copy)]
struct Named(&'static str);

impl Event for Named {
    fn name(&self) -> &'static str {
        self.0
    }
}

#[test]
fn version_is_non_empty() {
    assert_ne!(version(), "");
}

#[test]
fn test_kernel_boots_in_test_environment() {
    let mut app = SerenadeTestKernel::new();
    app.register_bundle(EmptyBundle).expect("register");
    app.boot().expect("boot");
    assert_eq!(app.kernel().phase(), KernelPhase::Booted);
    assert!(app.kernel().environment().is_debug());
    app.shutdown().expect("shutdown");
}

#[test]
fn http_client_get_and_post() {
    let kernel = HttpKernel::new(|request: &mut Request| {
        if request.method() == Method::Post {
            return Ok(Response::text(201, "created"));
        }
        Ok(Response::text(200, "ok"))
    });
    let client = HttpTestClient::new(&kernel);
    assert_eq!(client.get("/").status(), 200);
    assert_eq!(client.post("/", b"x").status(), 201);
}

#[test]
fn recording_subscriber_assert_dispatched() {
    let names = Arc::new(Mutex::new(Vec::new()));
    let mut dispatcher = EventDispatcher::new();
    dispatcher.add(Arc::new(RecordingSubscriber::new(
        "ping",
        Arc::clone(&names),
    )));
    dispatcher.dispatch(&Named("ping")).expect("dispatch");
    let recorded = names.lock().expect("lock").clone();
    assert_dispatched(&recorded, &["ping"]);
}
