//! Bind and run Actix Web with a Serenade [`AsyncHttpKernel`].

use std::net::ToSocketAddrs;

use actix_web::body::MessageBody;
use actix_web::dev::{ServiceFactory, ServiceRequest, ServiceResponse};
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use serenade_http::AsyncHttpKernel;

use crate::dispatch;

/// Builds the Actix app that forwards every request to `kernel` via [`dispatch::dispatch_async`].
#[must_use]
pub fn app(
    kernel: web::Data<AsyncHttpKernel>,
) -> App<
    impl ServiceFactory<
        ServiceRequest,
        Config = (),
        Response = ServiceResponse<impl MessageBody>,
        Error = Error,
        InitError = (),
    >,
> {
    App::new()
        .app_data(kernel)
        .default_service(web::to(serenade_service))
}

/// Binds `addr` and returns the Actix [`actix_web::dev::Server`] (does not await).
///
/// Prefer [`listen`] for the usual "run until stop" path. Use this when the caller
/// needs a [`actix_web::dev::ServerHandle`] for graceful shutdown in tests.
///
/// # Errors
///
/// Propagates bind errors.
pub fn bind_server(
    addr: impl ToSocketAddrs,
    kernel: AsyncHttpKernel,
) -> std::io::Result<actix_web::dev::Server> {
    let data = web::Data::new(kernel);
    Ok(HttpServer::new(move || app(data.clone())).bind(addr)?.run())
}

/// Awaits a server from [`bind_server`] until it stops.
///
/// # Errors
///
/// Propagates server IO errors.
#[allow(clippy::future_not_send)]
pub async fn await_bound(server: actix_web::dev::Server) -> std::io::Result<()> {
    server.await
}

/// Binds `addr` and serves every request through `kernel`.
///
/// Prefer this over hand-rolling `HttpServer::bind` in app skeletons.
/// Controllers may be async ([`AsyncHttpKernel`]); wrap sync handlers with
/// [`AsyncHttpKernel::from_sync`](serenade_http::AsyncHttpKernel::from_sync).
///
/// # Errors
///
/// Propagates bind and server IO errors.
///
/// Actix's server future is intentionally `!Send` (same as typical Actix handlers).
#[allow(clippy::future_not_send)]
pub async fn listen(addr: impl ToSocketAddrs, kernel: AsyncHttpKernel) -> std::io::Result<()> {
    await_bound(bind_server(addr, kernel)?).await
}

/// Actix `HttpRequest` is `!Send`; clippy nursery flags the resulting future.
#[allow(clippy::future_not_send)]
async fn serenade_service(
    request: HttpRequest,
    body: web::Bytes,
    kernel: web::Data<AsyncHttpKernel>,
) -> HttpResponse {
    dispatch::dispatch_async(kernel.get_ref(), &request, body).await
}
