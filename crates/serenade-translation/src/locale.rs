//! BCP 47 locale tags used by the translator and negotiator.

use std::fmt::{Display, Formatter};
use std::str::FromStr;

use crate::TranslationError;

/// Language tag (`en`, `fr`, `fr-FR`, …).
///
/// Tags are normalized to lowercase language and uppercase region when present
/// (`fr-fr` → `fr-FR`), matching common Symfony / ICU habits.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct Locale {
    tag: String,
}

impl Locale {
    /// Parses and normalizes a language tag.
    ///
    /// # Errors
    ///
    /// Returns [`TranslationError::InvalidLocale`] when `raw` is empty or has an
    /// empty language subtag.
    pub fn new(raw: impl AsRef<str>) -> Result<Self, TranslationError> {
        let raw = raw.as_ref().trim();
        if raw.is_empty() {
            return Err(TranslationError::InvalidLocale {
                tag: raw.to_owned(),
            });
        }
        let mut parts = raw.split('-');
        let language = parts.next().unwrap_or("").trim().to_ascii_lowercase();
        if language.is_empty() || !language.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err(TranslationError::InvalidLocale {
                tag: raw.to_owned(),
            });
        }
        let mut tag = language;
        if let Some(region) = parts.next() {
            let region = region.trim();
            if !region.is_empty() {
                if !region.chars().all(|c| c.is_ascii_alphanumeric()) {
                    return Err(TranslationError::InvalidLocale {
                        tag: raw.to_owned(),
                    });
                }
                tag.push('-');
                tag.push_str(&region.to_ascii_uppercase());
            }
        }
        Ok(Self { tag })
    }

    /// Full normalized tag.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.tag
    }

    /// Primary language subtag (`fr` for `fr-FR`).
    #[must_use]
    pub fn language(&self) -> &str {
        self.tag.split('-').next().unwrap_or(self.tag.as_str())
    }

    /// Region subtag when present (`FR` for `fr-FR`).
    #[must_use]
    pub fn region(&self) -> Option<&str> {
        self.tag.split('-').nth(1)
    }

    /// Fallback chain excluding `self`: `fr-FR` → `fr`.
    #[must_use]
    pub fn parent_chain(&self) -> Vec<Self> {
        let mut chain = Vec::new();
        if self.region().is_some() {
            if let Ok(parent) = Self::new(self.language()) {
                chain.push(parent);
            }
        }
        chain
    }
}

impl Display for Locale {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.tag)
    }
}

impl FromStr for Locale {
    type Err = TranslationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl AsRef<str> for Locale {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
