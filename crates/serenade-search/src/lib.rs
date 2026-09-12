//! Basic document search contracts and an in-memory [`MemorySearchAdapter`].
//!
//! Serenade ships index traits and a zero-deps memory adapter only. Product
//! engines (Postgres FTS, Meilisearch, and similar) are application adapters.

mod document;
mod error;
mod index;
mod memory;
mod query;

pub use document::SearchDocument;
pub use error::SearchError;
pub use index::DocumentIndex;
pub use memory::MemorySearchAdapter;
pub use query::{SearchHit, SearchQuery};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
