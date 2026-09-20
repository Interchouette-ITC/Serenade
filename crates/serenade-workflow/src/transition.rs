//! Named transition between places.

/// Named edge from one or more places to one or more places.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transition {
    name: String,
    from: Vec<String>,
    to: Vec<String>,
}

impl Transition {
    /// Build a transition. `from` and `to` must be non-empty when added to a definition.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        from: impl IntoIterator<Item = impl Into<String>>,
        to: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            name: name.into(),
            from: from.into_iter().map(Into::into).collect(),
            to: to.into_iter().map(Into::into).collect(),
        }
    }

    /// Transition name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Source places that must all be marked.
    #[must_use]
    pub fn from(&self) -> &[String] {
        &self.from
    }

    /// Destination places applied after leaving `from`.
    #[must_use]
    pub fn to(&self) -> &[String] {
        &self.to
    }
}
