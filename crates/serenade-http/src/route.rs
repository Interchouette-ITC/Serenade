//! Named route definition and collection.

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
