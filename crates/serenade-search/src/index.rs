//! Document index trait.

use crate::{SearchDocument, SearchError, SearchHit, SearchQuery};

/// Upsert / delete / query surface for searchable documents.
///
/// Adapters own ranking. Applications map domain entities into
/// [`SearchDocument`] and call this trait after writes.
///
/// # Examples
///
/// ```
/// use serenade_search::{DocumentIndex, MemorySearchAdapter, SearchDocument, SearchQuery};
///
/// let index = MemorySearchAdapter::new();
/// index
///     .upsert(SearchDocument::new("1").field("body", "hello serenade"))
///     .expect("upsert");
/// let hits = index.query(&SearchQuery::new("serenade")).expect("query");
/// assert_eq!(hits[0].id(), "1");
/// ```
pub trait DocumentIndex: Send + Sync {
    /// Inserts or replaces `document` by id.
    ///
    /// # Errors
    ///
    /// Returns [`SearchError::InvalidId`] when the id is empty, or
    /// [`SearchError::Index`] when the adapter cannot store the document.
    fn upsert(&self, document: SearchDocument) -> Result<(), SearchError>;

    /// Removes `id` when present. Returns `true` when a document was deleted.
    ///
    /// # Errors
    ///
    /// Returns [`SearchError::InvalidId`] when the id is empty, or
    /// [`SearchError::Index`] when delete fails.
    fn delete(&self, id: &str) -> Result<bool, SearchError>;

    /// Ranks documents that match `query`.
    ///
    /// # Errors
    ///
    /// Returns [`SearchError::InvalidQuery`] when the query is rejected, or
    /// [`SearchError::Index`] when the adapter cannot run the query.
    fn query(&self, query: &SearchQuery) -> Result<Vec<SearchHit>, SearchError>;

    /// Removes every document.
    ///
    /// # Errors
    ///
    /// Returns [`SearchError::Index`] when clear fails.
    fn clear(&self) -> Result<(), SearchError>;
}
