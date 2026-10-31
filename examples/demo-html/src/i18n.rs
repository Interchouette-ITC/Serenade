//! UI chrome strings resolved through Serenade Translator.

use serenade_translation::{Locale, Translator, TranslatorInterface};

/// Translated chrome for one request (post bodies stay author language).
#[derive(Clone, Debug)]
pub struct Ui<'a> {
    translator: &'a Translator,
    locale: Locale,
}

impl<'a> Ui<'a> {
    /// Binds translator + active locale.
    #[must_use]
    pub const fn new(translator: &'a Translator, locale: Locale) -> Self {
        Self { translator, locale }
    }

    /// Active locale.
    #[must_use]
    pub const fn locale(&self) -> &Locale {
        &self.locale
    }

    /// Translates `id` with no parameters.
    #[must_use]
    pub fn t(&self, id: &str) -> String {
        self.translator.trans(id, &[], None, Some(&self.locale))
    }

    /// Plural-aware translation.
    #[must_use]
    pub fn tn(&self, id: &str, number: i64) -> String {
        self.translator
            .trans_choice(id, number, &[], None, Some(&self.locale))
    }
}
