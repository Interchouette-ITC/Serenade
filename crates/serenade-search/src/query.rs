//! Query and hit types.

/// Text query against a [`DocumentIndex`](crate::DocumentIndex).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchQuery {
    text: String,
    limit: Option<usize>,
    offset: usize,
}

impl SearchQuery {
    /// Builds a query from free text.
    ///
    /// # Examples
    ///
    /// ```
    /// use serenade_search::SearchQuery;
    ///
    /// let q = SearchQuery::new("rust framework").with_limit(10).with_offset(0);
    /// assert_eq!(q.text(), "rust framework");
    /// assert_eq!(q.limit(), Some(10));
    /// assert_eq!(q.offset(), 0);
    /// ```
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            limit: None,
            offset: 0,
        }
    }

    /// Caps the number of hits returned (after offset).
    #[must_use]
    pub const fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Skips the first `offset` hits (after ranking).
    #[must_use]
    pub const fn with_offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    /// Query text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Optional hit limit.
    #[must_use]
    pub const fn limit(&self) -> Option<usize> {
        self.limit
    }

    /// Hit offset.
    #[must_use]
    pub const fn offset(&self) -> usize {
        self.offset
    }
}

/// Ranked match returned by [`DocumentIndex::query`](crate::DocumentIndex::query).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    id: String,
    score: u32,
}

impl SearchHit {
    /// Builds a hit for `id` with `score` (higher is better).
    #[must_use]
    pub fn new(id: impl Into<String>, score: u32) -> Self {
        Self {
            id: id.into(),
            score,
        }
    }

    /// Document id.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Ranking score from the adapter.
    #[must_use]
    pub const fn score(&self) -> u32 {
        self.score
    }
}
