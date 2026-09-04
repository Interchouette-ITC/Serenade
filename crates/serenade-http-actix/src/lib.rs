//! Actix-web bridge for [`serenade_http`].
//!
//! Convert Actix requests into Serenade [`Request`](serenade_http::Request) values,
//! run [`HttpKernel`](serenade_http::HttpKernel), then map the Serenade response
//! back to Actix. An Axum adapter can follow the same shape without changing
//! the foundation crate.
//!
//! Typical handler:
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
//! [`listen`] instead of wiring `HttpServer` by hand.
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
pub use dispatch::dispatch;
pub use listen::{app, await_bound, bind_server, listen};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
