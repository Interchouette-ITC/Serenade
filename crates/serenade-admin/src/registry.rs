//! Registry of admin resources.

use crate::AdminResource;
use crate::error::AdminError;

/// Holds configured [`AdminResource`] values.
#[derive(Clone, Debug, Default)]
pub struct AdminRegistry {
    resources: Vec<AdminResource>,
}

impl AdminRegistry {
    /// Empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a resource.
    ///
    /// # Errors
    ///
    /// Returns [`AdminError::DuplicateResource`] when `name` is already present.
    pub fn register(&mut self, resource: AdminResource) -> Result<(), AdminError> {
        if self
            .resources
            .iter()
            .any(|existing| existing.name() == resource.name())
        {
            return Err(AdminError::DuplicateResource {
                name: resource.name().to_owned(),
            });
        }
        self.resources.push(resource);
        Ok(())
    }

    /// All resources in registration order.
    #[must_use]
    pub fn resources(&self) -> &[AdminResource] {
        &self.resources
    }

    /// Looks up by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&AdminResource> {
        self.resources
            .iter()
            .find(|resource| resource.name() == name)
    }
}
