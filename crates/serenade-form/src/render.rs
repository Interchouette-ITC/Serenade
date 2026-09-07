//! Rendered HTML form output.

/// Escaped HTML produced by [`crate::Form::render`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedForm {
    pub(crate) html: String,
}

impl RenderedForm {
    /// Full `<form>…</form>` markup (values already escaped).
    #[must_use]
    pub fn as_html(&self) -> &str {
        &self.html
    }
}

impl AsRef<str> for RenderedForm {
    fn as_ref(&self) -> &str {
        &self.html
    }
}
