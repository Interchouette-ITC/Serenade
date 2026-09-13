//! Scripted client for unit tests.

use std::future::ready;
use std::sync::Mutex;

use crate::{ClientMethod, ClientRequest, ClientResponse, HttpClient, HttpClientError};

type Matcher = Box<dyn Fn(&ClientRequest) -> bool + Send + Sync>;

struct Expectation {
    matcher: Matcher,
    response: ClientResponse,
    remaining: Option<usize>,
}

/// In-memory [`HttpClient`] that returns scripted responses.
#[derive(Default)]
pub struct MockHttpClient {
    expectations: Mutex<Vec<Expectation>>,
}

impl MockHttpClient {
    /// Empty mock (misses until expectations are added).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Expects exact method + URL once (or forever when `times` is `None`).
    ///
    /// # Panics
    ///
    /// Panics when the internal mutex is poisoned.
    pub fn expect(
        &self,
        method: ClientMethod,
        url: impl Into<String>,
        response: ClientResponse,
        times: Option<usize>,
    ) -> &Self {
        let url = url.into();
        self.expectations
            .lock()
            .expect("mock lock")
            .push(Expectation {
                matcher: Box::new(move |request| {
                    request.method() == method && request.url() == url
                }),
                response,
                remaining: times,
            });
        self
    }

    /// Expects any request matching `matcher`.
    ///
    /// # Panics
    ///
    /// Panics when the internal mutex is poisoned.
    pub fn expect_where(
        &self,
        matcher: impl Fn(&ClientRequest) -> bool + Send + Sync + 'static,
        response: ClientResponse,
        times: Option<usize>,
    ) -> &Self {
        self.expectations
            .lock()
            .expect("mock lock")
            .push(Expectation {
                matcher: Box::new(matcher),
                response,
                remaining: times,
            });
        self
    }

    fn resolve(&self, request: &ClientRequest) -> Result<ClientResponse, HttpClientError> {
        let mut expectations =
            self.expectations
                .lock()
                .map_err(|_| HttpClientError::Transport {
                    message: "mock mutex poisoned".to_owned(),
                })?;
        let position = expectations.iter().position(|item| (item.matcher)(request));
        let Some(index) = position else {
            return Err(HttpClientError::MockMiss {
                method: request.method().to_string(),
                url: request.url().to_owned(),
            });
        };
        let response = expectations[index].response.clone();
        if let Some(left) = expectations[index].remaining {
            if left <= 1 {
                expectations.remove(index);
            } else {
                expectations[index].remaining = Some(left - 1);
            }
        }
        drop(expectations);
        Ok(response)
    }
}

impl HttpClient for MockHttpClient {
    fn send(
        &self,
        request: ClientRequest,
    ) -> impl std::future::Future<Output = Result<ClientResponse, HttpClientError>> + Send {
        ready(self.resolve(&request))
    }
}
