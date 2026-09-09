//! Internationalization and localization for Serenade applications.
//!
//! Framework-owned Translator, TOML-first catalogues, locale negotiation, and
//! ICU format helpers. Product multilang entity rows stay in the application.
//!
//! See `docs-dev/I18N.md`.

mod catalogue;
mod error;
mod extension;
mod format;
mod loader;
mod locale;
mod message;
mod middleware;
mod negotiate;
mod translator;

pub use catalogue::{DEFAULT_DOMAIN, INTL_DOMAIN_SUFFIX, MessageCatalogue};
pub use error::TranslationError;
pub use extension::{
    TRANSLATION_BUNDLE, TRANSLATOR_SERVICE, TranslationBundle, TranslationExtension,
};
pub use format::{format_currency, format_date, format_number, format_number_f64};
pub use loader::{
    CatalogueFileName, CatalogueLoader, JsonCatalogueLoader, TomlCatalogueLoader, load_directory,
    load_paths,
};
pub use locale::Locale;
pub use message::format_message;
pub use middleware::LocaleMiddleware;
pub use negotiate::{
    DEFAULT_LOCALE_COOKIE, DEFAULT_LOCALE_QUERY, LOCALE_ATTRIBUTE, LocaleNegotiator, request_locale,
};
pub use translator::{Translator, TranslatorInterface};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
