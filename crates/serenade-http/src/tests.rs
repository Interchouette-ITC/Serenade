use std::sync::{Arc, Mutex};

use super::{
    box_future, load_routes, AsyncHttpKernel, DefaultExceptionHandler, ExceptionHandler, HttpError,
    HttpKernel, Method, Middleware, Request, RequestHandler, Response, Route, RouteCollection,
    RouteLoader, UrlMatcher, ROUTE_ATTRIBUTE,
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
fn method_as_str_and_display_cover_all_variants() {
    let cases = [
        (Method::Get, "GET"),
        (Method::Head, "HEAD"),
        (Method::Post, "POST"),
        (Method::Put, "PUT"),
        (Method::Patch, "PATCH"),
        (Method::Delete, "DELETE"),
        (Method::Options, "OPTIONS"),
    ];
    for (method, token) in cases {
        assert_eq!(method.as_str(), token);
        assert_eq!(method.to_string(), token);
        assert_eq!(token.parse::<Method>().unwrap(), method);
    }
}

#[test]
fn headers_are_case_insensitive() {
    let mut headers = super::Headers::new();
    assert!(headers.is_empty());
    headers.insert("Content-Type", "application/json");
    assert_eq!(headers.get("content-type"), Some("application/json"));
    assert_eq!(headers.len(), 1);
    assert!(!headers.is_empty());
    assert!(headers.iter().any(|(name, _)| name == "content-type"));
}

#[test]
fn attributes_roundtrip() {
    let mut request = Request::new(Method::Get, "/items/1");
    request.attributes_mut().insert("item_id", 1_u64);
    assert_eq!(request.attributes().get::<u64>("item_id"), Some(&1));
    assert!(request.attributes().contains("item_id"));
    assert_eq!(request.attributes().len(), 1);
    assert_eq!(request.attributes_mut().remove::<u64>("item_id"), Some(1));
    assert!(request.attributes().is_empty());
    let debug = format!("{:?}", request.attributes());
    assert!(debug.contains("AttributeBag"));
}

#[test]
fn route_collection_rejects_duplicate_and_allows_any_method() {
    let route = Route::new("any", "/ping", []);
    assert_eq!(route.methods(), []);
    assert!(route.allows(Method::Delete));
    assert_eq!(route.path(), "/ping");
    let mut collection = RouteCollection::new();
    assert_eq!(collection.len(), 0);
    collection.add(route).expect("add");
    let err = collection
        .add(Route::with_method("any", "/other", Method::Get))
        .expect_err("duplicate");
    assert!(err.to_string().contains("already registered"));
    assert_eq!(collection.routes().len(), 1);
}

#[test]
fn matcher_returns_404_and_exposes_collection() {
    let mut collection = RouteCollection::new();
    collection
        .add(Route::with_method("item", "/items/{id}", Method::Get))
        .expect("add");
    let matcher = UrlMatcher::new(collection);
    assert_eq!(matcher.collection().len(), 1);
    let missing = matcher
        .match_request(Method::Get, "/nope")
        .expect_err("404");
    assert_eq!(missing.status_code(), 404);
    let wrong_len = matcher
        .match_request(Method::Get, "/items/1/extra")
        .expect_err("segment count");
    assert_eq!(wrong_len.status_code(), 404);
}

#[test]
fn closure_route_loader_registers_routes() {
    let collection = load_routes(&[&|routes: &mut RouteCollection| {
        routes.add(Route::with_method("closure", "/closure", Method::Get))
    }])
    .expect("load");
    assert_eq!(collection.len(), 1);
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
        .add(Route::with_method("item_show", "/items/{id}", Method::Get))
        .expect("add");
    let matcher = UrlMatcher::new(collection);
    let found = matcher
        .match_request(Method::Get, "/items/42")
        .expect("match");
    assert_eq!(found.route_name(), "item_show");
    assert_eq!(found.parameters().get("id").map(String::as_str), Some("42"));

    let mut request = Request::new(Method::Get, "/items/7");
    let applied = matcher.apply(&mut request).expect("apply");
    assert_eq!(applied.route_name(), "item_show");
    assert_eq!(
        request.attributes().get::<String>("id").map(String::as_str),
        Some("7")
    );
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

#[test]
fn http_error_convenience_constructors() {
    let not_found = HttpError::not_found("gone");
    assert_eq!(not_found.status_code(), 404);
    assert_eq!(not_found.message(), "gone");
    let bad = HttpError::bad_request("nope");
    assert_eq!(bad.status_code(), 400);
    let unprocessable = HttpError::unprocessable("bad body");
    assert_eq!(unprocessable.status_code(), 422);
    assert!(unprocessable.to_string().contains("422"));
}

#[test]
fn request_with_header_and_body_builders() {
    let request = Request::new(Method::Post, "/echo")
        .with_header("X-Trace", "abc")
        .with_body(b"ping".as_slice());
    assert_eq!(request.method(), Method::Post);
    assert_eq!(request.path(), "/echo");
    assert_eq!(request.headers().get("x-trace"), Some("abc"));
    assert_eq!(request.body(), b"ping");
}

#[test]
fn matcher_rejects_static_segment_mismatch() {
    let mut collection = RouteCollection::new();
    collection
        .add(Route::with_method("fixed", "/shop/cart", Method::Get))
        .expect("add");
    let matcher = UrlMatcher::new(collection);
    let err = matcher
        .match_request(Method::Get, "/shop/other")
        .expect_err("static mismatch");
    assert_eq!(err.status_code(), 404);
}

#[tokio::test]
async fn async_kernel_from_sync_serves() {
    let kernel = AsyncHttpKernel::from_sync(|request: &mut Request| {
        assert_eq!(request.path(), "/healthz");
        Ok(Response::text(200, "ok"))
    });
    let response = kernel.handle(Request::new(Method::Get, "/healthz")).await;
    assert_eq!(response.status(), 200);
    assert_eq!(response.body_str(), Some("ok"));
}

#[tokio::test]
async fn async_kernel_awaits_controller() {
    let kernel = AsyncHttpKernel::from_async_fn(|request: &mut Request| {
        let path = request.path().to_owned();
        box_future(async move { Ok(Response::text(200, path)) })
    });
    let response = kernel.handle(Request::new(Method::Get, "/async")).await;
    assert_eq!(response.status(), 200);
    assert_eq!(response.body_str(), Some("/async"));
}

#[tokio::test]
async fn async_kernel_maps_errors() {
    let kernel = AsyncHttpKernel::from_async_fn(|_: &mut Request| {
        box_future(async { Err(HttpError::status(404, "gone")) })
    });
    let response = kernel.handle(Request::new(Method::Get, "/x")).await;
    assert_eq!(response.status(), 404);
    assert_eq!(response.body_str(), Some("gone"));
}
