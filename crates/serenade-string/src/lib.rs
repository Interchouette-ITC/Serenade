//! Slug, case transforms, and English Inflector helpers (Symfony String shaped).
//!
//! - [`slug`] - URL-safe lowercase slug
//! - [`snake_case`], [`camel_case`], [`pascal_case`], [`kebab_case`], [`title_case`]
//! - [`pluralize`], [`singularize`] - English rules with documented limits

mod case;
mod inflect;
mod slug;

pub use case::{camel_case, kebab_case, pascal_case, snake_case, title_case};
pub use inflect::{pluralize, singularize};
pub use slug::slug;

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
