//! Actix-web bridge for [`serenade_http`].
//!
//! Convert Actix requests into Serenade [`Request`](serenade_http::Request) values,
//! run a kernel ([`HttpKernel`](serenade_http::HttpKernel) sync or
//! [`AsyncHttpKernel`](serenade_http::AsyncHttpKernel)), then map the Serenade
//! response back to Actix. An Axum adapter can follow the same shape without
//! changing the foundation crate.
//!
//! Typical sync handler:
//!
//! ```ignore
//! async fn healthz(
//!     request: HttpRequest,
//!     body: web::Bytes,
//!     kernel: web::Data<HttpKernel>,
//! ) -> HttpResponse {
//!     dispatch(kernel.get_ref(), &request, body)
//! }
//! ```
//!
//! App skeletons that only need “bind and serve the kernel” can call
//! [`listen`] with an [`AsyncHttpKernel`](serenade_http::AsyncHttpKernel)
//! (use [`from_sync`](serenade_http::AsyncHttpKernel::from_sync) for sync
//! controllers) instead of wiring `HttpServer` by hand.
//!
//! # Examples
//!
//! ```
//! use serenade_http::Response;
//! use serenade_http_actix::to_actix;
//!
//! let response = to_actix(&Response::text(200, "ok"));
//! assert_eq!(response.status(), 200);
//! ```

mod convert;
mod dispatch;
mod listen;

pub use convert::{conversion_error, from_actix, to_actix};
pub use dispatch::{dispatch, dispatch_async};
pub use listen::{app, await_bound, bind_server, listen};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
