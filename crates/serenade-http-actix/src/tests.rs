use actix_web::test as actix_test;
use actix_web::{web, App, HttpRequest, HttpResponse};
use serenade_http::{HttpKernel, Method, Request, Response};

use super::{
    app, await_bound, bind_server, conversion_error, dispatch, from_actix, listen, to_actix,
    version,
};

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
async fn listen_app_default_service_dispatches_kernel() {
    let kernel = HttpKernel::new(|request: &mut Request| {
        assert_eq!(request.path(), "/any");
        Ok(Response::text(200, "served"))
    });
    let service = actix_test::init_service(app(web::Data::new(kernel))).await;
    let request = actix_test::TestRequest::get().uri("/any").to_request();
    let response = actix_test::call_service(&service, request).await;
    assert!(response.status().is_success());
    let body = actix_test::read_body(response).await;
    assert_eq!(body.as_ref(), b"served");
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

#[test]
fn conversion_error_maps_to_405_response() {
    let error = serenade_http::HttpError::status(405, "method not allowed");
    let response = conversion_error(&error);
    assert_eq!(response.status(), 405);
}

#[actix_web::test]
async fn dispatch_maps_from_actix_errors() {
    let kernel = HttpKernel::new(|_: &mut Request| Ok(Response::text(200, "ok")));
    let request = actix_test::TestRequest::default()
        .method(actix_web::http::Method::TRACE)
        .uri("/")
        .to_http_request();
    let response = dispatch(&kernel, &request, []);
    assert_eq!(response.status(), 405);
}

#[actix_web::test]
async fn listen_binds_serves_then_stops() {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::time::Duration;

    let probe = TcpListener::bind("127.0.0.1:0").expect("bind probe");
    let addr = probe.local_addr().expect("addr");
    drop(probe);

    let kernel = HttpKernel::new(|_request: &mut Request| Ok(Response::text(200, "listen-ok")));
    let server = bind_server(addr, kernel).expect("bind");
    let handle = server.handle();
    let server = actix_web::rt::spawn(server);

    let mut body = None;
    for _ in 0..50 {
        actix_web::rt::time::sleep(Duration::from_millis(20)).await;
        let Ok(mut stream) = TcpStream::connect(addr) else {
            continue;
        };
        stream
            .set_read_timeout(Some(Duration::from_millis(200)))
            .expect("read timeout");
        stream
            .set_write_timeout(Some(Duration::from_millis(200)))
            .expect("write timeout");
        let request = format!("GET / HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
        if stream.write_all(request.as_bytes()).is_err() {
            continue;
        }
        let mut buf = Vec::new();
        if stream.read_to_end(&mut buf).is_err() || buf.is_empty() {
            continue;
        }
        body = Some(buf);
        break;
    }

    handle.stop(true).await;
    server.await.expect("join").expect("server stopped cleanly");
    let body = body.expect("listen did not become ready");
    let text = String::from_utf8_lossy(&body);
    assert!(text.contains("200"), "{text}");
    assert!(text.contains("listen-ok"), "{text}");
}

#[actix_web::test]
async fn listen_awaits_until_server_stops() {
    use std::net::TcpListener;
    use std::time::Duration;

    let probe = TcpListener::bind("127.0.0.1:0").expect("bind probe");
    let addr = probe.local_addr().expect("addr");
    drop(probe);

    let kernel = HttpKernel::new(|_request: &mut Request| Ok(Response::text(200, "ok")));
    let server = bind_server(addr, kernel).expect("bind");
    let handle = server.handle();
    let task = actix_web::rt::spawn(async move { await_bound(server).await });
    actix_web::rt::time::sleep(Duration::from_millis(40)).await;
    handle.stop(true).await;
    task.await.expect("join").expect("listen await completed");
}

#[actix_web::test]
async fn listen_public_entry_binds_and_serves() {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::time::Duration;

    let probe = TcpListener::bind("127.0.0.1:0").expect("bind probe");
    let addr = probe.local_addr().expect("addr");
    drop(probe);

    let kernel = HttpKernel::new(|_request: &mut Request| Ok(Response::text(200, "entry-ok")));
    let task = actix_web::rt::spawn(async move { listen(addr, kernel).await });

    let mut body = None;
    for _ in 0..50 {
        actix_web::rt::time::sleep(Duration::from_millis(20)).await;
        let Ok(mut stream) = TcpStream::connect(addr) else {
            continue;
        };
        stream
            .set_read_timeout(Some(Duration::from_millis(200)))
            .expect("read timeout");
        stream
            .set_write_timeout(Some(Duration::from_millis(200)))
            .expect("write timeout");
        let request = format!("GET / HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
        if stream.write_all(request.as_bytes()).is_err() {
            continue;
        }
        let mut buf = Vec::new();
        if stream.read_to_end(&mut buf).is_err() || buf.is_empty() {
            continue;
        }
        body = Some(buf);
        break;
    }

    assert!(
        body.as_ref()
            .is_some_and(|b| String::from_utf8_lossy(b).contains("entry-ok")),
        "listen entry did not become ready"
    );
    task.abort();
    let _ = task.await;
}

#[actix_web::test]
async fn listen_propagates_bind_error() {
    let kernel = HttpKernel::new(|_request: &mut Request| Ok(Response::text(200, "unused")));
    // Privileged port fails for non-root runners (CI and local).
    let err = listen("127.0.0.1:1", kernel).await;
    assert!(err.is_err(), "expected bind failure on privileged port");
}
