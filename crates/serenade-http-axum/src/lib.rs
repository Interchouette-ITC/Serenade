//! Axum bridge for [`serenade_http`].
//!
//! Convert Axum/HTTP requests into Serenade [`Request`](serenade_http::Request) values,
//! run a kernel ([`HttpKernel`](serenade_http::HttpKernel) sync or
//! [`AsyncHttpKernel`](serenade_http::AsyncHttpKernel)), then map the Serenade
//! response back to Axum. Same four-step bridge as the Actix adapter.
//!
//! App skeletons that only need “bind and serve the kernel” can call
//! [`listen`] with an [`AsyncHttpKernel`](serenade_http::AsyncHttpKernel)
//! (use [`from_sync`](serenade_http::AsyncHttpKernel::from_sync) for sync
//! controllers) instead of wiring `axum::serve` by hand.
//!
//! # Examples
//!
//! ```
//! use serenade_http::Response;
//! use serenade_http_axum::to_axum;
//!
//! let response = to_axum(&Response::text(200, "ok"));
//! assert_eq!(response.status(), 200);
//! ```

mod convert;
mod dispatch;
mod listen;

pub use convert::{conversion_error, from_axum, to_axum};
pub use dispatch::{dispatch, dispatch_async};
pub use listen::{BoundServer, ShutdownHandle, await_bound, bind_server, listen, router};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
