//! Translator facade (Symfony `TranslatorInterface` analogue).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;

use crate::catalogue::DEFAULT_DOMAIN;
use crate::loader::{
    CatalogueLoader, JsonCatalogueLoader, TomlCatalogueLoader, load_directory, load_paths,
};
use crate::message::format_message;
use crate::{Locale, MessageCatalogue, TranslationError};

/// Read-only translation contract for apps and middleware.
pub trait TranslatorInterface: Send + Sync {
    /// Active locale used when callers omit an override.
    fn locale(&self) -> Locale;

    /// Translates `id` in `domain` (default [`DEFAULT_DOMAIN`]).
    ///
    /// Missing messages return `id` (Symfony habit).
    fn trans(
        &self,
        id: &str,
        parameters: &[(&str, &str)],
        domain: Option<&str>,
        locale: Option<&Locale>,
    ) -> String;

    /// Translates a pluralizable message; injects `count` into parameters.
    fn trans_choice(
        &self,
        id: &str,
        number: i64,
        parameters: &[(&str, &str)],
        domain: Option<&str>,
        locale: Option<&Locale>,
    ) -> String;
}

/// In-memory translator with fallback locales and pluggable catalogue loaders.
///
/// # Examples
///
/// ```
/// use serenade_translation::{Locale, MessageCatalogue, Translator, TranslatorInterface};
///
/// let mut en = MessageCatalogue::new(Locale::new("en").unwrap());
/// en.set("messages", "hello", "Hello {name}");
/// let mut translator = Translator::new(Locale::new("en").unwrap());
/// translator.add_catalogue(en);
/// assert_eq!(
///     translator.trans("hello", &[("name", "Ada")], None, None),
///     "Hello Ada"
/// );
/// ```
#[derive(Debug)]
pub struct Translator {
    locale: RwLock<Locale>,
    fallbacks: Vec<Locale>,
    catalogues: HashMap<String, MessageCatalogue>,
}

impl Translator {
    /// Translator with `default_locale` and no catalogues yet.
    #[must_use]
    pub fn new(default_locale: Locale) -> Self {
        Self {
            locale: RwLock::new(default_locale),
            fallbacks: Vec::new(),
            catalogues: HashMap::new(),
        }
    }

    /// Sets explicit fallback locales (tried after the request locale parent chain).
    #[must_use]
    pub fn with_fallbacks(mut self, fallbacks: Vec<Locale>) -> Self {
        self.fallbacks = fallbacks;
        self
    }

    /// Replaces the active locale.
    pub fn set_locale(&self, locale: Locale) {
        if let Ok(mut guard) = self.locale.write() {
            *guard = locale;
        }
    }

    /// Poisons the locale lock (test helper for recovery coverage).
    #[cfg(test)]
    pub(crate) fn poison_locale_lock_for_test(&self) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = self.locale.write().expect("locale write");
            panic!("poison locale lock");
        }));
    }

    /// Inserts or replaces a catalogue for its locale.
    pub fn add_catalogue(&mut self, catalogue: MessageCatalogue) {
        let key = catalogue.locale().as_str().to_owned();
        if let Some(existing) = self.catalogues.get_mut(&key) {
            existing.replace_catalogue(&catalogue);
        } else {
            self.catalogues.insert(key, catalogue);
        }
    }

    /// Loads TOML + JSON catalogues from `dir`.
    ///
    /// # Errors
    ///
    /// Returns [`TranslationError`] when directory load fails.
    pub fn load_path(&mut self, dir: impl AsRef<std::path::Path>) -> Result<(), TranslationError> {
        let toml = TomlCatalogueLoader;
        let json = JsonCatalogueLoader;
        let loaders: [&dyn CatalogueLoader; 2] = [&toml, &json];
        for (_, catalogue) in load_directory(dir, &loaders)? {
            self.add_catalogue(catalogue);
        }
        Ok(())
    }

    /// Loads catalogues from multiple directories (later paths override).
    ///
    /// # Errors
    ///
    /// Returns [`TranslationError`] when any directory load fails.
    pub fn load_paths(&mut self, paths: &[PathBuf]) -> Result<(), TranslationError> {
        let toml = TomlCatalogueLoader;
        let json = JsonCatalogueLoader;
        let loaders: [&dyn CatalogueLoader; 2] = [&toml, &json];
        for (_, catalogue) in load_paths(paths, &loaders)? {
            self.add_catalogue(catalogue);
        }
        Ok(())
    }

    /// Number of loaded locale catalogues.
    #[must_use]
    pub fn catalogue_count(&self) -> usize {
        self.catalogues.len()
    }

    fn resolve_template(&self, id: &str, domain: &str, locale: &Locale) -> Option<String> {
        for candidate in self.lookup_chain(locale) {
            if let Some(catalogue) = self.catalogues.get(candidate.as_str()) {
                if let Some(template) = catalogue.get(id, domain) {
                    return Some(template.to_owned());
                }
            }
        }
        None
    }

    fn lookup_chain(&self, locale: &Locale) -> Vec<Locale> {
        let mut chain = vec![locale.clone()];
        chain.extend(locale.parent_chain());
        for fallback in &self.fallbacks {
            if !chain.iter().any(|item| item == fallback) {
                chain.push(fallback.clone());
            }
            for parent in fallback.parent_chain() {
                if !chain.iter().any(|existing| existing == &parent) {
                    chain.push(parent);
                }
            }
        }
        chain
    }
}

impl TranslatorInterface for Translator {
    fn locale(&self) -> Locale {
        match self.locale.read() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    fn trans(
        &self,
        id: &str,
        parameters: &[(&str, &str)],
        domain: Option<&str>,
        locale: Option<&Locale>,
    ) -> String {
        let domain = domain.unwrap_or(DEFAULT_DOMAIN);
        let active = self.locale();
        let locale = locale.unwrap_or(&active);
        let Some(template) = self.resolve_template(id, domain, locale) else {
            return id.to_owned();
        };
        format_message(&template, parameters, locale, None)
    }

    fn trans_choice(
        &self,
        id: &str,
        number: i64,
        parameters: &[(&str, &str)],
        domain: Option<&str>,
        locale: Option<&Locale>,
    ) -> String {
        let domain = domain.unwrap_or(DEFAULT_DOMAIN);
        let active = self.locale();
        let locale = locale.unwrap_or(&active);
        let Some(template) = self.resolve_template(id, domain, locale) else {
            return id.to_owned();
        };
        let count = number.to_string();
        let mut owned: Vec<(&str, &str)> = parameters.to_vec();
        if !owned.iter().any(|(key, _)| *key == "count") {
            owned.push(("count", count.as_str()));
        }
        format_message(&template, &owned, locale, Some(number))
    }
}

impl TranslatorInterface for std::sync::Arc<Translator> {
    fn locale(&self) -> Locale {
        (**self).locale()
    }

    fn trans(
        &self,
        id: &str,
        parameters: &[(&str, &str)],
        domain: Option<&str>,
        locale: Option<&Locale>,
    ) -> String {
        (**self).trans(id, parameters, domain, locale)
    }

    fn trans_choice(
        &self,
        id: &str,
        number: i64,
        parameters: &[(&str, &str)],
        domain: Option<&str>,
        locale: Option<&Locale>,
    ) -> String {
        (**self).trans_choice(id, number, parameters, domain, locale)
    }
}
