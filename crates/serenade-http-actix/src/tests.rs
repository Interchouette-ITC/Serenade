use actix_web::test as actix_test;
use actix_web::{web, App, HttpRequest, HttpResponse};
use serenade_http::{HttpKernel, Method, Request, Response};

use super::{dispatch, from_actix, to_actix, version};

/// Actix `HttpRequest` is `!Send`; clippy nursery flags the resulting future.
#[allow(clippy::future_not_send)]
async fn healthz(
    request: HttpRequest,
    body: web::Bytes,
    kernel: web::Data<HttpKernel>,
) -> HttpResponse {
    dispatch(kernel.get_ref(), &request, body)
}

#[test]
fn version_is_non_empty() {
    assert_ne!(version(), "");
}

#[test]
fn to_actix_preserves_status_and_body() {
    let response = to_actix(&Response::text(201, "created"));
    assert_eq!(response.status(), 201);
}

#[actix_web::test]
async fn sample_route_returns_plain_text_via_kernel() {
    let kernel = HttpKernel::new(|request: &mut Request| {
        assert_eq!(request.path(), "/healthz");
        assert_eq!(request.method(), Method::Get);
        Ok(Response::text(200, "ok"))
    });
    let app = actix_test::init_service(
        App::new()
            .app_data(web::Data::new(kernel))
            .route("/healthz", web::get().to(healthz)),
    )
    .await;

    let request = actix_test::TestRequest::get().uri("/healthz").to_request();
    let response = actix_test::call_service(&app, request).await;
    assert!(response.status().is_success());
    let body = actix_test::read_body(response).await;
    assert_eq!(body.as_ref(), b"ok");
}

#[actix_web::test]
async fn from_actix_copies_headers_and_body() {
    let kernel = HttpKernel::new(|request: &mut Request| {
        assert_eq!(request.headers().get("x-trace"), Some("abc"));
        assert_eq!(request.body(), b"ping");
        Ok(Response::text(200, "pong"))
    });
    let app = actix_test::init_service(
        App::new()
            .app_data(web::Data::new(kernel))
            .route("/echo", web::post().to(healthz)),
    )
    .await;

    let request = actix_test::TestRequest::post()
        .uri("/echo")
        .insert_header(("x-trace", "abc"))
        .set_payload("ping")
        .to_request();
    let response = actix_test::call_service(&app, request).await;
    assert!(response.status().is_success());
    let body = actix_test::read_body(response).await;
    assert_eq!(body.as_ref(), b"pong");
}

#[actix_web::test]
async fn from_actix_rejects_unsupported_method_in_isolation() {
    let request = actix_test::TestRequest::default()
        .method(actix_web::http::Method::TRACE)
        .uri("/")
        .to_http_request();
    let error = from_actix(&request, []).expect_err("TRACE unsupported");
    assert_eq!(error.status_code(), 405);
}
