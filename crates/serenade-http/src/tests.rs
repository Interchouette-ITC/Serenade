use std::sync::{Arc, Mutex};

use super::{
    load_routes, DefaultExceptionHandler, ExceptionHandler, HttpError, HttpKernel, Method,
    Middleware, Request, RequestHandler, Response, Route, RouteCollection, RouteLoader, UrlMatcher,
    ROUTE_ATTRIBUTE,
};

struct TraceLayer {
    name: &'static str,
    log: Arc<Mutex<Vec<&'static str>>>,
}

impl Middleware for TraceLayer {
    fn process(
        &self,
        request: &mut Request,
        next: &dyn RequestHandler,
    ) -> Result<Response, HttpError> {
        self.log.lock().expect("log").push(self.name);
        next.handle(request)
    }
}

struct FailMapper;

impl ExceptionHandler for FailMapper {
    fn handle(&self, error: &HttpError) -> Response {
        Response::text(503, format!("mapped:{}", error.message()))
    }
}

#[test]
fn version_is_non_empty() {
    assert_ne!(super::version(), "");
}

#[test]
fn method_parses_ascii_case() {
    assert_eq!("post".parse::<Method>().unwrap(), Method::Post);
    assert!("TRACE".parse::<Method>().is_err());
}

#[test]
fn headers_are_case_insensitive() {
    let mut headers = super::Headers::new();
    headers.insert("Content-Type", "application/json");
    assert_eq!(headers.get("content-type"), Some("application/json"));
    assert_eq!(headers.len(), 1);
}

#[test]
fn attributes_roundtrip() {
    let mut request = Request::new(Method::Get, "/carts/1");
    request.attributes_mut().insert("cart_id", 1_u64);
    assert_eq!(request.attributes().get::<u64>("cart_id"), Some(&1));
    assert_eq!(request.attributes_mut().remove::<u64>("cart_id"), Some(1));
    assert!(request.attributes().is_empty());
}

#[test]
fn kernel_runs_controller() {
    let kernel = HttpKernel::new(|request: &mut Request| {
        assert_eq!(request.path(), "/healthz");
        Ok(Response::text(200, "ok"))
    });
    let response = kernel.handle(Request::new(Method::Get, "/healthz"));
    assert_eq!(response.status(), 200);
    assert_eq!(response.body_str(), Some("ok"));
}

#[test]
fn middleware_runs_outer_first() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut kernel = HttpKernel::new({
        let log = Arc::clone(&log);
        move |_request: &mut Request| {
            log.lock().expect("log").push("controller");
            Ok(Response::new(204))
        }
    });
    kernel.push_middleware(TraceLayer {
        name: "outer",
        log: Arc::clone(&log),
    });
    kernel.push_middleware(TraceLayer {
        name: "inner",
        log: Arc::clone(&log),
    });
    let response = kernel.handle(Request::new(Method::Get, "/"));
    assert_eq!(response.status(), 204);
    assert_eq!(*log.lock().expect("log"), ["outer", "inner", "controller"]);
}

#[test]
fn default_exception_handler_maps_status() {
    let kernel = HttpKernel::new(|_request: &mut Request| Err(HttpError::status(404, "missing")));
    let response = kernel.handle(Request::new(Method::Get, "/gone"));
    assert_eq!(response.status(), 404);
    assert_eq!(response.body_str(), Some("missing"));
    let fallback = DefaultExceptionHandler.handle(&HttpError::failed("boom"));
    assert_eq!(fallback.status(), 500);
}

#[test]
fn custom_exception_handler_replaces_default() {
    let kernel = HttpKernel::new(|_request: &mut Request| Err(HttpError::failed("nope")))
        .with_exception_handler(FailMapper);
    let response = kernel.handle(Request::new(Method::Post, "/"));
    assert_eq!(response.status(), 503);
    assert_eq!(response.body_str(), Some("mapped:nope"));
}

#[test]
fn matcher_extracts_path_parameters() {
    let mut collection = RouteCollection::new();
    collection
        .add(Route::with_method("cart_show", "/carts/{id}", Method::Get))
        .expect("add");
    let matcher = UrlMatcher::new(collection);
    let found = matcher
        .match_request(Method::Get, "/carts/42")
        .expect("match");
    assert_eq!(found.route_name(), "cart_show");
    assert_eq!(found.parameters().get("id").map(String::as_str), Some("42"));
}

#[test]
fn matcher_returns_405_for_wrong_method() {
    let mut collection = RouteCollection::new();
    collection
        .add(Route::with_method("healthz", "/healthz", Method::Get))
        .expect("add");
    let matcher = UrlMatcher::new(collection);
    let error = matcher
        .match_request(Method::Post, "/healthz")
        .expect_err("method");
    assert_eq!(error.status_code(), 405);
}

#[test]
fn bundle_loader_registers_get_healthz() {
    let collection = load_routes(&[&HealthzBundle]).expect("load");
    assert_eq!(collection.len(), 1);
    let matcher = UrlMatcher::new(collection);
    let mut request = Request::new(Method::Get, "/healthz");
    let found = matcher.apply(&mut request).expect("apply");
    assert_eq!(found.route_name(), "healthz");
    assert_eq!(
        request
            .attributes()
            .get::<String>(ROUTE_ATTRIBUTE)
            .map(String::as_str),
        Some("healthz")
    );

    let kernel = HttpKernel::new(|req: &mut Request| {
        match req
            .attributes()
            .get::<String>(ROUTE_ATTRIBUTE)
            .map(String::as_str)
        {
            Some("healthz") => Ok(Response::text(200, "ok")),
            _ => Err(HttpError::status(404, "missing")),
        }
    });
    let response = kernel.handle(request);
    assert_eq!(response.status(), 200);
    assert_eq!(response.body_str(), Some("ok"));
}

struct HealthzBundle;

impl RouteLoader for HealthzBundle {
    fn load(&self, collection: &mut RouteCollection) -> Result<(), HttpError> {
        collection.add(Route::with_method("healthz", "/healthz", Method::Get))
    }
}
