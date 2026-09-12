//! HTTP foundation: request, response, attributes, middleware, routing, and kernel.
//!
//! Server adapters (Actix, Axum, and others) stay thin wrappers over this layer.

mod async_handler;
mod async_kernel;
mod async_middleware;
mod attributes;
mod error;
mod exception;
mod handler;
mod headers;
mod http_cache;
mod kernel;
mod loader;
mod matcher;
mod method;
mod middleware;
mod request;
mod response;
mod route;

pub use async_handler::{AsyncFn, AsyncRequestHandler, BoxFuture, SyncToAsync, box_future};
pub use async_kernel::AsyncHttpKernel;
pub use async_middleware::{AsyncMiddleware, AsyncNext};
pub use attributes::AttributeBag;
pub use error::HttpError;
pub use exception::{DefaultExceptionHandler, ExceptionHandler};
pub use handler::RequestHandler;
pub use headers::Headers;
pub use http_cache::{HttpCacheHeaders, etag, maybe_not_modified, weak_etag};
pub use kernel::HttpKernel;
pub use loader::{RouteLoader, load_routes};
pub use matcher::{MatchResult, ROUTE_ATTRIBUTE, UrlMatcher};
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
