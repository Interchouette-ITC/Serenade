//! Path and method matching against a [`crate::RouteCollection`].

use std::collections::HashMap;

use crate::{HttpError, Method, RouteCollection};

/// Attribute key written by [`UrlMatcher::apply`] for the matched route name.
pub const ROUTE_ATTRIBUTE: &str = "_route";

/// Successful match: route name plus path parameters.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchResult {
    route_name: String,
    parameters: HashMap<String, String>,
}

impl MatchResult {
    /// Matched route name.
    #[must_use]
    pub fn route_name(&self) -> &str {
        &self.route_name
    }

    /// Captured path parameters.
    #[must_use]
    pub const fn parameters(&self) -> &HashMap<String, String> {
        &self.parameters
    }
}

/// Matches request method and path against a [`RouteCollection`].
#[derive(Clone, Debug, Default)]
pub struct UrlMatcher {
    collection: RouteCollection,
}

impl UrlMatcher {
    /// Matcher over `collection`.
    #[must_use]
    pub const fn new(collection: RouteCollection) -> Self {
        Self { collection }
    }

    /// Underlying collection.
    #[must_use]
    pub const fn collection(&self) -> &RouteCollection {
        &self.collection
    }

    /// Finds the first route whose path matches and that allows `method`.
    ///
    /// # Errors
    ///
    /// - Status **404** when no path matches.
    /// - Status **405** when a path matches but the method is not allowed.
    pub fn match_request(&self, method: Method, path: &str) -> Result<MatchResult, HttpError> {
        let mut path_matched = false;
        for route in self.collection.routes() {
            let Some(parameters) = match_path(route.path(), path) else {
                continue;
            };
            path_matched = true;
            if route.allows(method) {
                return Ok(MatchResult {
                    route_name: route.name().to_owned(),
                    parameters,
                });
            }
        }
        if path_matched {
            Err(HttpError::status(
                405,
                format!("method `{method}` not allowed"),
            ))
        } else {
            Err(HttpError::status(404, format!("no route for `{path}`")))
        }
    }

    /// Writes `_route` and path parameters onto `request` attributes.
    ///
    /// # Errors
    ///
    /// Propagates [`Self::match_request`] errors.
    pub fn apply(&self, request: &mut crate::Request) -> Result<MatchResult, HttpError> {
        let matched = self.match_request(request.method(), request.path())?;
        request
            .attributes_mut()
            .insert(ROUTE_ATTRIBUTE, matched.route_name().to_owned());
        for (key, value) in matched.parameters() {
            request.attributes_mut().insert(key.clone(), value.clone());
        }
        Ok(matched)
    }
}

fn match_path(pattern: &str, path: &str) -> Option<HashMap<String, String>> {
    let pattern_segments = split_segments(pattern);
    let path_segments = split_segments(path);
    if pattern_segments.len() != path_segments.len() {
        return None;
    }
    let mut parameters = HashMap::new();
    for (expected, actual) in pattern_segments.into_iter().zip(path_segments) {
        if let Some(name) = parameter_name(expected) {
            parameters.insert(name.to_owned(), actual.to_owned());
        } else if expected != actual {
            return None;
        }
    }
    Some(parameters)
}

fn split_segments(path: &str) -> Vec<&str> {
    path.trim_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect()
}

fn parameter_name(segment: &str) -> Option<&str> {
    segment
        .strip_prefix('{')
        .and_then(|rest| rest.strip_suffix('}'))
        .filter(|name| !name.is_empty())
}
