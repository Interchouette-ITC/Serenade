//! In-memory [`DocumentIndex`](crate::DocumentIndex).

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use crate::{DocumentIndex, SearchDocument, SearchError, SearchHit, SearchQuery};

/// Zero-deps in-memory index with case-insensitive token substring matching.
///
/// Query text is split on Unicode whitespace. A document matches when every
/// non-empty token appears as a substring of the concatenated field values
/// (lowercased). Score is the sum of token occurrence counts. Empty query text
/// (or only whitespace) returns no hits.
#[derive(Default)]
pub struct MemorySearchAdapter {
    inner: Mutex<HashMap<String, SearchDocument>>,
}

impl MemorySearchAdapter {
    /// Creates an empty index.
    ///
    /// # Examples
    ///
    /// ```
    /// use serenade_search::{DocumentIndex, MemorySearchAdapter, SearchDocument, SearchQuery};
    ///
    /// let index = MemorySearchAdapter::new();
    /// index
    ///     .upsert(SearchDocument::new("a").field("title", "MyFeed post"))
    ///     .expect("upsert");
    /// let hits = index
    ///     .query(&SearchQuery::new("myfeed").with_limit(5))
    ///     .expect("query");
    /// assert_eq!(hits.len(), 1);
    /// assert_eq!(hits[0].id(), "a");
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn validate_id(id: &str) -> Result<(), SearchError> {
        if id.is_empty() {
            return Err(SearchError::InvalidId {
                id: id.to_owned(),
                message: "id must not be empty".to_owned(),
            });
        }
        Ok(())
    }

    fn lock_map(&self) -> MutexGuard<'_, HashMap<String, SearchDocument>> {
        self.inner
            .lock()
            .expect("serenade-search MemorySearchAdapter mutex poisoned")
    }

    fn tokens(text: &str) -> Vec<String> {
        text.split_whitespace()
            .map(str::to_lowercase)
            .filter(|token| !token.is_empty())
            .collect()
    }

    fn haystack(document: &SearchDocument) -> String {
        let mut parts = Vec::with_capacity(document.fields().len());
        for value in document.fields().values() {
            parts.push(value.to_lowercase());
        }
        parts.join(" ")
    }

    fn score(haystack: &str, tokens: &[String]) -> Option<u32> {
        if tokens.is_empty() {
            return None;
        }
        let mut total = 0_u32;
        for token in tokens {
            let count = Self::count_occurrences(haystack, token);
            if count == 0 {
                return None;
            }
            total = total.saturating_add(count);
        }
        Some(total)
    }

    fn count_occurrences(haystack: &str, needle: &str) -> u32 {
        if needle.is_empty() {
            return 0;
        }
        let mut count = 0_u32;
        let mut rest = haystack;
        while let Some(pos) = rest.find(needle) {
            count = count.saturating_add(1);
            rest = &rest[pos + needle.len()..];
        }
        count
    }

    fn apply_page(mut hits: Vec<SearchHit>, query: &SearchQuery) -> Vec<SearchHit> {
        hits.sort_by(|left, right| {
            right
                .score()
                .cmp(&left.score())
                .then_with(|| left.id().cmp(right.id()))
        });
        let skipped = hits.into_iter().skip(query.offset());
        match query.limit() {
            Some(limit) => skipped.take(limit).collect(),
            None => skipped.collect(),
        }
    }
}

impl DocumentIndex for MemorySearchAdapter {
    fn upsert(&self, document: SearchDocument) -> Result<(), SearchError> {
        Self::validate_id(document.id())?;
        let id = document.id().to_owned();
        self.lock_map().insert(id, document);
        Ok(())
    }

    fn delete(&self, id: &str) -> Result<bool, SearchError> {
        Self::validate_id(id)?;
        Ok(self.lock_map().remove(id).is_some())
    }

    fn query(&self, query: &SearchQuery) -> Result<Vec<SearchHit>, SearchError> {
        let tokens = Self::tokens(query.text());
        if tokens.is_empty() {
            return Ok(Vec::new());
        }
        let map = self.lock_map();
        let mut hits = Vec::new();
        for document in map.values() {
            let haystack = Self::haystack(document);
            if let Some(score) = Self::score(&haystack, &tokens) {
                hits.push(SearchHit::new(document.id(), score));
            }
        }
        drop(map);
        Ok(Self::apply_page(hits, query))
    }

    fn clear(&self) -> Result<(), SearchError> {
        self.lock_map().clear();
        Ok(())
    }
}

#[cfg(test)]
mod private_path_tests {
    use super::MemorySearchAdapter;

    #[test]
    fn score_returns_none_for_empty_tokens() {
        assert_eq!(MemorySearchAdapter::score("anything", &[]), None);
    }

    #[test]
    fn count_occurrences_returns_zero_for_empty_needle() {
        assert_eq!(MemorySearchAdapter::count_occurrences("abc", ""), 0);
    }
}
