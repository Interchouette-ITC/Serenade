//! Service reference used when declaring dependencies.

/// Points at another service id in the container.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Reference {
    id: String,
}

impl Reference {
    /// Creates a reference to `id`.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }

    /// Referenced service id.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
}

impl From<&str> for Reference {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for Reference {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}
