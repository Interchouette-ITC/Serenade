//! HTTP foundation: request, response, attributes, middleware, routing, and kernel.
//!
//! Server adapters (Actix, Axum, and others) stay thin wrappers over this layer.

mod async_handler;
mod async_kernel;
mod attributes;
mod error;
mod exception;
mod handler;
mod headers;
mod kernel;
mod loader;
mod matcher;
mod method;
mod middleware;
mod request;
mod response;
mod route;

pub use async_handler::{box_future, AsyncFn, AsyncRequestHandler, BoxFuture, SyncToAsync};
pub use async_kernel::AsyncHttpKernel;
pub use attributes::AttributeBag;
pub use error::HttpError;
pub use exception::{DefaultExceptionHandler, ExceptionHandler};
pub use handler::RequestHandler;
pub use headers::Headers;
pub use kernel::HttpKernel;
pub use loader::{load_routes, RouteLoader};
pub use matcher::{MatchResult, UrlMatcher, ROUTE_ATTRIBUTE};
pub use method::Method;
pub use middleware::Middleware;
pub use request::Request;
pub use response::Response;
pub use route::{Route, RouteCollection};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
