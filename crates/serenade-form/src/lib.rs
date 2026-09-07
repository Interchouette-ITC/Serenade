//! HTML form component: bind request, CSRF by default, HTML escape on render.
//!
//! Apps declare fields and constraints; Serenade owns CSRF validation and XSS-safe
//! escaping. See `docs-dev/FORMS.md` and `docs-dev/SECURITY.md`.

mod error;
mod escape;
mod form;
mod parse;
mod render;

pub use error::FormError;
pub use escape::{escape_attr, escape_html};
pub use form::{Field, Form, FormBuilder, FormStatus};
pub use parse::parse_urlencoded;
pub use render::RenderedForm;
pub use serenade_security::CSRF_FIELD_NAME;

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
