//! Named route definition and collection.

use std::collections::HashMap;

use crate::Method;

/// A single route: name, path pattern, and allowed methods.
///
/// Path segments wrapped in `{…}` are parameters (for example `/items/{id}`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Route {
    name: String,
    path: String,
    methods: Vec<Method>,
}

impl Route {
    /// Creates a route. Empty `methods` means any method matches.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        path: impl Into<String>,
        methods: impl IntoIterator<Item = Method>,
    ) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            methods: methods.into_iter().collect(),
        }
    }

    /// Convenience for a single-method route.
    #[must_use]
    pub fn with_method(name: impl Into<String>, path: impl Into<String>, method: Method) -> Self {
        Self::new(name, path, [method])
    }

    /// Route name (unique within a collection).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Path pattern.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Allowed methods. Empty means any method.
    #[must_use]
    pub fn methods(&self) -> &[Method] {
        &self.methods
    }

    /// Whether `method` is allowed.
    #[must_use]
    pub fn allows(&self, method: Method) -> bool {
        self.methods.is_empty() || self.methods.contains(&method)
    }
}

/// Ordered list of routes. First match wins.
#[derive(Clone, Debug, Default)]
pub struct RouteCollection {
    routes: Vec<Route>,
}

impl RouteCollection {
    /// Empty collection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a route.
    ///
    /// # Errors
    ///
    /// Returns [`crate::HttpError`] when another route already uses the same name.
    pub fn add(&mut self, route: Route) -> Result<(), crate::HttpError> {
        if self
            .routes
            .iter()
            .any(|existing| existing.name() == route.name())
        {
            return Err(crate::HttpError::failed(format!(
                "route `{}` is already registered",
                route.name()
            )));
        }
        self.routes.push(route);
        Ok(())
    }

    /// Looks up a route by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Route> {
        self.routes.iter().find(|route| route.name() == name)
    }

    /// Builds a path for the named route by substituting `{param}` segments.
    ///
    /// Values are percent-encoded as path segments. Every pattern parameter must
    /// be provided; unused keys are rejected.
    ///
    /// # Errors
    ///
    /// - Status **404** when `name` is not registered.
    /// - Status **400** when a required parameter is missing or an unused key remains.
    pub fn generate(
        &self,
        name: &str,
        params: &[(&str, &str)],
    ) -> Result<String, crate::HttpError> {
        let route = self
            .get(name)
            .ok_or_else(|| crate::HttpError::status(404, format!("no route named `{name}`")))?;
        let mut unused: HashMap<&str, &str> = HashMap::with_capacity(params.len());
        for &(key, value) in params {
            if unused.insert(key, value).is_some() {
                return Err(crate::HttpError::status(
                    400,
                    format!("duplicate parameter `{key}` for route `{name}`"),
                ));
            }
        }
        let segments = split_segments(route.path());
        let mut out = Vec::with_capacity(segments.len());
        for segment in segments {
            if let Some(param) = parameter_name(segment) {
                let Some(value) = unused.remove(param) else {
                    return Err(crate::HttpError::status(
                        400,
                        format!("missing parameter `{param}` for route `{name}`"),
                    ));
                };
                out.push(encode_path_segment(value));
            } else {
                out.push(segment.to_owned());
            }
        }
        if let Some((extra, _)) = unused.iter().next() {
            return Err(crate::HttpError::status(
                400,
                format!("unused parameter `{extra}` for route `{name}`"),
            ));
        }
        if out.is_empty() {
            return Ok("/".to_owned());
        }
        Ok(format!("/{}", out.join("/")))
    }

    /// Registered routes in order.
    #[must_use]
    pub fn routes(&self) -> &[Route] {
        &self.routes
    }

    /// Number of routes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.routes.len()
    }

    /// Whether the collection is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }
}

pub fn split_segments(path: &str) -> Vec<&str> {
    path.trim_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn parameter_name(segment: &str) -> Option<&str> {
    segment
        .strip_prefix('{')
        .and_then(|rest| rest.strip_suffix('}'))
        .filter(|name| !name.is_empty())
}

fn encode_path_segment(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(byte));
            }
            _ => {
                let _ = std::fmt::Write::write_fmt(&mut out, format_args!("%{byte:02X}"));
            }
        }
    }
    out
}
