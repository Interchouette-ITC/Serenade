//! Service scope and definition metadata.

use crate::Reference;

/// Lifetime of a resolved service instance.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Scope {
    /// One shared instance for the container lifetime.
    Singleton,
    /// Fresh instance on every resolve.
    Prototype,
}

/// Registered service definition before and after compile.
#[derive(Clone, Debug)]
pub struct ServiceDefinition {
    id: String,
    scope: Scope,
    dependencies: Vec<Reference>,
    tags: Vec<String>,
}

impl ServiceDefinition {
    /// Creates a singleton definition with no declared dependencies.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            scope: Scope::Singleton,
            dependencies: Vec::new(),
            tags: Vec::new(),
        }
    }

    /// Sets the service scope.
    #[must_use]
    pub const fn with_scope(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }

    /// Declares dependencies resolved before the factory runs.
    #[must_use]
    pub fn with_dependencies(mut self, dependencies: Vec<Reference>) -> Self {
        self.dependencies = dependencies;
        self
    }

    /// Adds a service tag (for example `event.subscriber`).
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Service id.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Service scope.
    #[must_use]
    pub const fn scope(&self) -> Scope {
        self.scope
    }

    /// Declared dependencies.
    #[must_use]
    pub fn dependencies(&self) -> &[Reference] {
        &self.dependencies
    }

    /// Service tags used by compile passes.
    #[must_use]
    pub fn tags(&self) -> &[String] {
        &self.tags
    }
}
