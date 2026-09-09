//! Message catalogue for one locale (domains → ids → templates).

use std::collections::HashMap;

use crate::Locale;

/// Default Symfony-shaped domain name.
pub const DEFAULT_DOMAIN: &str = "messages";

/// Suffix Symfony uses for ICU `MessageFormat` domains (`messages+intl-icu`).
pub const INTL_DOMAIN_SUFFIX: &str = "+intl-icu";

/// In-memory message bag for a single locale.
#[derive(Clone, Debug)]
pub struct MessageCatalogue {
    locale: Locale,
    /// domain → (id → template)
    messages: HashMap<String, HashMap<String, String>>,
}

impl MessageCatalogue {
    /// Empty catalogue for `locale`.
    #[must_use]
    pub fn new(locale: Locale) -> Self {
        Self {
            locale,
            messages: HashMap::new(),
        }
    }

    /// Catalogue locale.
    #[must_use]
    pub const fn locale(&self) -> &Locale {
        &self.locale
    }

    /// Inserts or replaces one message.
    pub fn set(
        &mut self,
        domain: impl Into<String>,
        id: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.messages
            .entry(domain.into())
            .or_default()
            .insert(id.into(), message.into());
    }

    /// Merges all messages from `other` (same locale). Existing ids win.
    pub fn add_catalogue(&mut self, other: &Self) {
        for (domain, messages) in &other.messages {
            let entry = self.messages.entry(domain.clone()).or_default();
            for (id, text) in messages {
                entry.entry(id.clone()).or_insert_with(|| text.clone());
            }
        }
    }

    /// Overwrites with messages from `other` (same locale).
    pub fn replace_catalogue(&mut self, other: &Self) {
        for (domain, messages) in &other.messages {
            let entry = self.messages.entry(domain.clone()).or_default();
            for (id, text) in messages {
                entry.insert(id.clone(), text.clone());
            }
        }
    }

    /// Looks up `id` in `domain`, then in `domain+intl-icu`.
    #[must_use]
    pub fn get(&self, id: &str, domain: &str) -> Option<&str> {
        self.messages
            .get(domain)
            .and_then(|m| m.get(id))
            .or_else(|| {
                let intl = format!("{domain}{INTL_DOMAIN_SUFFIX}");
                self.messages.get(&intl).and_then(|m| m.get(id))
            })
            .map(String::as_str)
    }

    /// Whether the id exists in domain or intl-icu sibling.
    #[must_use]
    pub fn has(&self, id: &str, domain: &str) -> bool {
        self.get(id, domain).is_some()
    }

    /// Domain names without the `+intl-icu` suffix duplicated.
    #[must_use]
    pub fn domains(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .messages
            .keys()
            .map(|domain| {
                domain
                    .strip_suffix(INTL_DOMAIN_SUFFIX)
                    .unwrap_or(domain)
                    .to_owned()
            })
            .collect();
        names.sort();
        names.dedup();
        names
    }

    /// All messages for `domain` (merged with intl-icu sibling).
    #[must_use]
    pub fn all(&self, domain: &str) -> HashMap<String, String> {
        let mut out = HashMap::new();
        if let Some(plain) = self.messages.get(domain) {
            out.extend(plain.clone());
        }
        let intl = format!("{domain}{INTL_DOMAIN_SUFFIX}");
        for (id, text) in self.messages.get(&intl).into_iter().flatten() {
            out.entry(id.clone()).or_insert_with(|| text.clone());
        }
        out
    }
}
