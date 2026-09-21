//! Basic document search contracts and an in-memory [`MemorySearchAdapter`].
//!
//! Serenade ships index traits and a zero-deps memory adapter. Feature `http`
//! adds `HttpSearchAdapter` (Meilisearch/ES-shaped REST; apps own the HTTP client).

mod document;
mod error;
mod index;
mod memory;
mod query;

#[cfg(feature = "http")]
mod http;

pub use document::SearchDocument;
pub use error::SearchError;
pub use index::DocumentIndex;
pub use memory::MemorySearchAdapter;
pub use query::{SearchHit, SearchQuery};

#[cfg(feature = "http")]
pub use http::{
    HttpSearchAdapter, HttpSearchConfig, MockSearchHttpPoster, MockSearchRequest, SearchHttpMethod,
    SearchHttpPoster, SearchHttpResponse,
};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
