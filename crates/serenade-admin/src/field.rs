//! Field descriptors for admin list / show.

/// One column / property shown in admin HTML.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminField {
    name: String,
    label: String,
}

impl AdminField {
    /// Creates a field; label defaults to `name` when omitted via [`Self::named`].
    #[must_use]
    pub fn new(name: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
        }
    }

    /// Field whose label equals its name.
    #[must_use]
    pub fn named(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            label: name.clone(),
            name,
        }
    }

    /// Property key (matches [`crate::AdminRow`] values).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Display label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }
}
