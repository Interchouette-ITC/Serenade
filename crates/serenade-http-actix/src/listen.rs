//! Bind and run Actix Web with a Serenade [`HttpKernel`].

use std::net::ToSocketAddrs;

use actix_web::body::MessageBody;
use actix_web::dev::{ServiceFactory, ServiceRequest, ServiceResponse};
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use serenade_http::HttpKernel;

use crate::dispatch;

/// Builds the Actix app that forwards every request to `kernel` via [`dispatch`].
#[must_use]
pub fn app(
    kernel: web::Data<HttpKernel>,
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

/// Binds `addr` and serves every request through `kernel`.
///
/// Prefer this over hand-rolling `HttpServer::bind` in app skeletons.
///
/// # Errors
///
/// Propagates bind and server IO errors.
///
/// Actix's server future is intentionally `!Send` (same as typical Actix handlers).
#[allow(clippy::future_not_send)]
pub async fn listen(addr: impl ToSocketAddrs, kernel: HttpKernel) -> std::io::Result<()> {
    let data = web::Data::new(kernel);
    HttpServer::new(move || app(data.clone()))
        .bind(addr)?
        .run()
        .await
}

/// Actix `HttpRequest` is `!Send`; clippy nursery flags the resulting future.
#[allow(clippy::future_not_send)]
async fn serenade_service(
    request: HttpRequest,
    body: web::Bytes,
    kernel: web::Data<HttpKernel>,
) -> HttpResponse {
    dispatch(kernel.get_ref(), &request, body)
}
