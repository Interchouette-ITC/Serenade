//! HTTP search stub adapter (feature `http`).
//!
//! Shapes a Meilisearch/ES-class REST index API and a sync HTTP poster trait so
//! apps can plug `ureq`, `reqwest` blocking, or another client without Serenade
//! owning a vendor SDK.

use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

use crate::{DocumentIndex, SearchDocument, SearchError, SearchHit, SearchQuery};

/// HTTP method used by [`SearchHttpPoster`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchHttpMethod {
    /// HTTP PUT.
    Put,
    /// HTTP POST.
    Post,
    /// HTTP DELETE.
    Delete,
}

impl SearchHttpMethod {
    /// Uppercase method name for logging / headers.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Put => "PUT",
            Self::Post => "POST",
            Self::Delete => "DELETE",
        }
    }
}

/// Base URL and optional Bearer API key for an HTTP search engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpSearchConfig {
    base_url: String,
    api_key: Option<String>,
}

impl HttpSearchConfig {
    /// Creates config for `base_url` (no trailing slash required).
    #[must_use]
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: trim_trailing_slash(base_url.into()),
            api_key: None,
        }
    }

    /// Sets an optional Bearer API key.
    #[must_use]
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Base URL without a trailing slash.
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Bearer API key when configured.
    #[must_use]
    pub fn api_key_value(&self) -> Option<&str> {
        self.api_key.as_deref()
    }

    /// Document URL for `id` (`{base}/documents/{id}`).
    #[must_use]
    pub fn document_url(&self, id: &str) -> String {
        format!("{}/documents/{}", self.base_url, id)
    }

    /// Documents collection URL (`{base}/documents`).
    #[must_use]
    pub fn documents_url(&self) -> String {
        format!("{}/documents", self.base_url)
    }

    /// Search URL (`{base}/search`).
    #[must_use]
    pub fn search_url(&self) -> String {
        format!("{}/search", self.base_url)
    }
}

/// HTTP response returned by a [`SearchHttpPoster`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHttpResponse {
    status: u16,
    body: String,
}

impl SearchHttpResponse {
    /// Creates a response with HTTP `status` and `body`.
    #[must_use]
    pub fn new(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            body: body.into(),
        }
    }

    /// HTTP status code.
    #[must_use]
    pub const fn status(&self) -> u16 {
        self.status
    }

    /// Response body text.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }

    /// Whether the status is in the 2xx success range.
    #[must_use]
    pub const fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }
}

/// Sync HTTP request used by [`HttpSearchAdapter`]. Apps implement this.
pub trait SearchHttpPoster: Send + Sync {
    /// Sends `method` to `url` with optional JSON `body`.
    ///
    /// # Errors
    ///
    /// Returns [`SearchError::Index`] when the HTTP call fails.
    fn request(
        &self,
        method: SearchHttpMethod,
        url: &str,
        headers: &[(&str, &str)],
        body: Option<&[u8]>,
    ) -> Result<SearchHttpResponse, SearchError>;
}

/// Captured request for [`MockSearchHttpPoster`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockSearchRequest {
    /// HTTP method.
    pub method: SearchHttpMethod,
    /// Request URL.
    pub url: String,
    /// Header pairs as owned strings.
    pub headers: Vec<(String, String)>,
    /// Raw body bytes when present.
    pub body: Option<Vec<u8>>,
}

/// Scripted poster for unit tests.
#[derive(Debug, Clone)]
pub struct MockSearchHttpPoster {
    response: Result<SearchHttpResponse, SearchError>,
    last: Arc<Mutex<Option<MockSearchRequest>>>,
}

impl MockSearchHttpPoster {
    /// Always returns `response`.
    #[must_use]
    pub fn new(response: SearchHttpResponse) -> Self {
        Self {
            response: Ok(response),
            last: Arc::new(Mutex::new(None)),
        }
    }

    /// Always returns HTTP 200 with an empty JSON object.
    #[must_use]
    pub fn ok_empty() -> Self {
        Self::new(SearchHttpResponse::new(200, "{}"))
    }

    /// Always fails the HTTP call with `error` (no response).
    #[must_use]
    pub fn failing(error: SearchError) -> Self {
        Self {
            response: Err(error),
            last: Arc::new(Mutex::new(None)),
        }
    }

    /// Last request observed by this mock, when any.
    #[must_use]
    pub fn last_request(&self) -> Option<MockSearchRequest> {
        self.last
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl SearchHttpPoster for MockSearchHttpPoster {
    fn request(
        &self,
        method: SearchHttpMethod,
        url: &str,
        headers: &[(&str, &str)],
        body: Option<&[u8]>,
    ) -> Result<SearchHttpResponse, SearchError> {
        let request = MockSearchRequest {
            method,
            url: url.to_owned(),
            headers: headers
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                .collect(),
            body: body.map(<[u8]>::to_vec),
        };
        *self
            .last
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(request);
        self.response.clone()
    }
}

/// HTTP search adapter implementing [`DocumentIndex`].
#[derive(Clone, Debug)]
pub struct HttpSearchAdapter<P> {
    config: HttpSearchConfig,
    poster: P,
}

impl<P> HttpSearchAdapter<P> {
    /// Creates an adapter with `config` and sync HTTP `poster`.
    #[must_use]
    pub const fn new(config: HttpSearchConfig, poster: P) -> Self {
        Self { config, poster }
    }

    /// Search API config.
    #[must_use]
    pub const fn config(&self) -> &HttpSearchConfig {
        &self.config
    }

    /// Underlying HTTP poster.
    #[must_use]
    pub const fn poster(&self) -> &P {
        &self.poster
    }

    fn headers<'a>(&'a self, auth: &'a mut Option<String>) -> Vec<(&'a str, &'a str)> {
        let mut headers = vec![("Content-Type", "application/json")];
        if let Some(key) = self.config.api_key_value() {
            *auth = Some(format!("Bearer {key}"));
        }
        if let Some(value) = auth.as_deref() {
            headers.push(("Authorization", value));
        }
        headers
    }

    fn require_success(response: &SearchHttpResponse) -> Result<(), SearchError> {
        if response.is_success() {
            return Ok(());
        }
        Err(SearchError::Index {
            message: format!(
                "search HTTP {} {}",
                response.status(),
                truncate_body(response.body())
            ),
        })
    }
}

impl<P: SearchHttpPoster> DocumentIndex for HttpSearchAdapter<P> {
    fn upsert(&self, document: SearchDocument) -> Result<(), SearchError> {
        validate_id(document.id())?;
        let payload = document_json(&document);
        let url = self.config.document_url(document.id());
        let mut auth = None;
        let headers = self.headers(&mut auth);
        let response = self.poster.request(
            SearchHttpMethod::Put,
            &url,
            &headers,
            Some(payload.as_slice()),
        )?;
        Self::require_success(&response)
    }

    fn delete(&self, id: &str) -> Result<bool, SearchError> {
        validate_id(id)?;
        let url = self.config.document_url(id);
        let mut auth = None;
        let headers = self.headers(&mut auth);
        let response = self
            .poster
            .request(SearchHttpMethod::Delete, &url, &headers, None)?;
        Self::require_success(&response)?;
        Ok(parse_deleted(response.body()).unwrap_or(true))
    }

    fn query(&self, query: &SearchQuery) -> Result<Vec<SearchHit>, SearchError> {
        let payload = query_json(query);
        let url = self.config.search_url();
        let mut auth = None;
        let headers = self.headers(&mut auth);
        let response = self.poster.request(
            SearchHttpMethod::Post,
            &url,
            &headers,
            Some(payload.as_slice()),
        )?;
        Self::require_success(&response)?;
        parse_hits(response.body())
    }

    fn clear(&self) -> Result<(), SearchError> {
        let url = self.config.documents_url();
        let mut auth = None;
        let headers = self.headers(&mut auth);
        let response = self
            .poster
            .request(SearchHttpMethod::Delete, &url, &headers, None)?;
        Self::require_success(&response)
    }
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

fn document_json(document: &SearchDocument) -> Vec<u8> {
    let fields: Value = document
        .fields()
        .iter()
        .map(|(key, value)| (key.clone(), Value::String(value.clone())))
        .collect::<serde_json::Map<String, Value>>()
        .into();
    let payload = json!({
        "id": document.id(),
        "fields": fields,
    });
    serde_json::to_vec(&payload).expect("search document JSON encode is infallible")
}

fn query_json(query: &SearchQuery) -> Vec<u8> {
    let mut payload = json!({
        "q": query.text(),
        "offset": query.offset(),
    });
    if let Some(limit) = query.limit() {
        payload["limit"] = json!(limit);
    }
    serde_json::to_vec(&payload).expect("search query JSON encode is infallible")
}

fn parse_deleted(body: &str) -> Option<bool> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return None;
    }
    let value: Value = serde_json::from_str(trimmed).ok()?;
    value.get("deleted").and_then(Value::as_bool)
}

fn parse_hits(body: &str) -> Result<Vec<SearchHit>, SearchError> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let value: Value = serde_json::from_str(trimmed).map_err(|error| SearchError::Index {
        message: format!("search HTTP JSON: {error}"),
    })?;
    let hits = value
        .get("hits")
        .and_then(Value::as_array)
        .ok_or_else(|| SearchError::Index {
            message: "search HTTP response missing hits array".to_owned(),
        })?;
    let mut out = Vec::with_capacity(hits.len());
    for hit in hits {
        let id = hit
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| SearchError::Index {
                message: "search hit missing id".to_owned(),
            })?;
        let score = hit
            .get("score")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .try_into()
            .unwrap_or(u32::MAX);
        out.push(SearchHit::new(id, score));
    }
    Ok(out)
}

fn trim_trailing_slash(mut url: String) -> String {
    while url.ends_with('/') {
        url.pop();
    }
    url
}

fn truncate_body(body: &str) -> String {
    const MAX: usize = 200;
    let trimmed = body.trim();
    if trimmed.len() <= MAX {
        return trimmed.to_owned();
    }
    format!("{}...", &trimmed[..MAX])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_doc() -> SearchDocument {
        SearchDocument::new("1").field("body", "hello serenade")
    }

    #[test]
    fn config_urls_and_api_key() {
        let config = HttpSearchConfig::new("https://search.example/")
            .api_key("tok")
            .api_key("tok-2");
        assert_eq!(config.base_url(), "https://search.example");
        assert_eq!(config.api_key_value(), Some("tok-2"));
        assert_eq!(
            config.document_url("1"),
            "https://search.example/documents/1"
        );
        assert_eq!(config.documents_url(), "https://search.example/documents");
        assert_eq!(config.search_url(), "https://search.example/search");
    }

    #[test]
    fn method_as_str() {
        assert_eq!(SearchHttpMethod::Put.as_str(), "PUT");
        assert_eq!(SearchHttpMethod::Post.as_str(), "POST");
        assert_eq!(SearchHttpMethod::Delete.as_str(), "DELETE");
    }

    #[test]
    fn response_accessors() {
        let response = SearchHttpResponse::new(201, " created ");
        assert_eq!(response.status(), 201);
        assert_eq!(response.body(), " created ");
        assert!(response.is_success());
        assert!(!SearchHttpResponse::new(404, "").is_success());
    }

    #[test]
    fn adapter_exposes_config_and_poster() {
        let poster = MockSearchHttpPoster::ok_empty();
        let adapter =
            HttpSearchAdapter::new(HttpSearchConfig::new("https://search.example"), poster);
        assert_eq!(adapter.config().base_url(), "https://search.example");
        assert!(adapter.poster().last_request().is_none());
    }

    #[test]
    fn upsert_puts_document_json() {
        let poster = MockSearchHttpPoster::ok_empty();
        let adapter = HttpSearchAdapter::new(
            HttpSearchConfig::new("https://search.example").api_key("k"),
            poster.clone(),
        );
        adapter.upsert(sample_doc()).expect("upsert");
        let request = poster.last_request().expect("captured");
        assert_eq!(request.method, SearchHttpMethod::Put);
        assert_eq!(request.url, "https://search.example/documents/1");
        assert!(
            request
                .headers
                .iter()
                .any(|(name, value)| name == "Authorization" && value == "Bearer k")
        );
        let body = request.body.expect("body");
        let value: Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(value["id"], "1");
        assert_eq!(value["fields"]["body"], "hello serenade");
    }

    #[test]
    fn delete_returns_deleted_flag() {
        let poster =
            MockSearchHttpPoster::new(SearchHttpResponse::new(200, r#"{"deleted":false}"#));
        let adapter =
            HttpSearchAdapter::new(HttpSearchConfig::new("https://search.example"), poster);
        assert!(!adapter.delete("1").expect("delete"));
    }

    #[test]
    fn delete_defaults_true_on_empty_body() {
        let poster = MockSearchHttpPoster::new(SearchHttpResponse::new(204, ""));
        let adapter =
            HttpSearchAdapter::new(HttpSearchConfig::new("https://search.example"), poster);
        assert!(adapter.delete("1").expect("delete"));
    }

    #[test]
    fn query_posts_and_parses_hits() {
        let poster = MockSearchHttpPoster::new(SearchHttpResponse::new(
            200,
            r#"{"hits":[{"id":"1","score":3},{"id":"2"}]}"#,
        ));
        let adapter = HttpSearchAdapter::new(
            HttpSearchConfig::new("https://search.example"),
            poster.clone(),
        );
        let hits = adapter
            .query(&SearchQuery::new("serenade").with_limit(5).with_offset(1))
            .expect("query");
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].id(), "1");
        assert_eq!(hits[0].score(), 3);
        assert_eq!(hits[1].id(), "2");
        assert_eq!(hits[1].score(), 0);
        let request = poster.last_request().expect("captured");
        assert_eq!(request.method, SearchHttpMethod::Post);
        assert_eq!(request.url, "https://search.example/search");
        let body = request.body.expect("body");
        let value: Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(value["q"], "serenade");
        assert_eq!(value["limit"], 5);
        assert_eq!(value["offset"], 1);
    }

    #[test]
    fn query_rejects_bad_json() {
        let poster = MockSearchHttpPoster::new(SearchHttpResponse::new(200, "not-json"));
        let adapter =
            HttpSearchAdapter::new(HttpSearchConfig::new("https://search.example"), poster);
        let err = adapter.query(&SearchQuery::new("x")).expect_err("bad json");
        assert!(matches!(err, SearchError::Index { .. }));
    }

    #[test]
    fn query_rejects_missing_hits() {
        let poster = MockSearchHttpPoster::new(SearchHttpResponse::new(200, r#"{"ok":true}"#));
        let adapter =
            HttpSearchAdapter::new(HttpSearchConfig::new("https://search.example"), poster);
        let err = adapter
            .query(&SearchQuery::new("x"))
            .expect_err("missing hits");
        assert!(matches!(err, SearchError::Index { .. }));
    }

    #[test]
    fn query_rejects_hit_without_id() {
        let poster =
            MockSearchHttpPoster::new(SearchHttpResponse::new(200, r#"{"hits":[{"score":1}]}"#));
        let adapter =
            HttpSearchAdapter::new(HttpSearchConfig::new("https://search.example"), poster);
        let err = adapter
            .query(&SearchQuery::new("x"))
            .expect_err("missing id");
        assert!(matches!(err, SearchError::Index { .. }));
    }

    #[test]
    fn clear_deletes_documents_collection() {
        let poster = MockSearchHttpPoster::ok_empty();
        let adapter = HttpSearchAdapter::new(
            HttpSearchConfig::new("https://search.example"),
            poster.clone(),
        );
        adapter.clear().expect("clear");
        let request = poster.last_request().expect("captured");
        assert_eq!(request.method, SearchHttpMethod::Delete);
        assert_eq!(request.url, "https://search.example/documents");
    }

    #[test]
    fn rejects_empty_id() {
        let poster = MockSearchHttpPoster::ok_empty();
        let adapter =
            HttpSearchAdapter::new(HttpSearchConfig::new("https://search.example"), poster);
        let err = adapter
            .upsert(SearchDocument::new(""))
            .expect_err("empty id");
        assert!(matches!(err, SearchError::InvalidId { .. }));
        let err = adapter.delete("").expect_err("empty id");
        assert!(matches!(err, SearchError::InvalidId { .. }));
    }

    #[test]
    fn rejects_non_2xx_and_truncates_body() {
        let long = "x".repeat(250);
        let poster = MockSearchHttpPoster::new(SearchHttpResponse::new(502, long));
        let adapter =
            HttpSearchAdapter::new(HttpSearchConfig::new("https://search.example"), poster);
        let err = adapter.upsert(sample_doc()).expect_err("fail");
        assert!(matches!(
            &err,
            SearchError::Index { message }
                if message.contains("502") && message.contains("...") && message.len() < 280
        ));
    }

    #[test]
    fn rejects_non_2xx_with_short_body() {
        let poster = MockSearchHttpPoster::new(SearchHttpResponse::new(503, "down"));
        let adapter =
            HttpSearchAdapter::new(HttpSearchConfig::new("https://search.example"), poster);
        let err = adapter.query(&SearchQuery::new("x")).expect_err("fail");
        assert!(matches!(
            &err,
            SearchError::Index { message } if message.contains("503") && message.contains("down")
        ));
    }

    #[test]
    fn propagates_poster_transport_error() {
        let poster = MockSearchHttpPoster::failing(SearchError::Index {
            message: "network down".to_owned(),
        });
        let adapter =
            HttpSearchAdapter::new(HttpSearchConfig::new("https://search.example"), poster);
        let err = adapter.upsert(sample_doc()).expect_err("fail");
        assert!(matches!(
            &err,
            SearchError::Index { message } if message == "network down"
        ));
        let err = adapter.query(&SearchQuery::new("x")).expect_err("fail");
        assert!(matches!(
            &err,
            SearchError::Index { message } if message == "network down"
        ));
    }

    #[test]
    fn query_empty_body_returns_no_hits() {
        let poster = MockSearchHttpPoster::new(SearchHttpResponse::new(200, "  "));
        let adapter =
            HttpSearchAdapter::new(HttpSearchConfig::new("https://search.example"), poster);
        let hits = adapter.query(&SearchQuery::new("x")).expect("query");
        assert_eq!(hits.len(), 0);
    }
}
