//! HTTP foundation: request, response, attributes, middleware, and kernel.
//!
//! Server adapters (Actix, Axum, and others) stay thin wrappers over this layer.

mod attributes;
mod error;
mod exception;
mod handler;
mod headers;
mod kernel;
mod method;
mod middleware;
mod request;
mod response;

pub use attributes::AttributeBag;
pub use error::HttpError;
pub use exception::{DefaultExceptionHandler, ExceptionHandler};
pub use handler::RequestHandler;
pub use headers::Headers;
pub use kernel::HttpKernel;
pub use method::Method;
pub use middleware::Middleware;
pub use request::Request;
pub use response::Response;

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
