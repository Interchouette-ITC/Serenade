//! Locale negotiation from query, cookie, Accept-Language, and defaults.

use serenade_http::Request;

use crate::Locale;

/// Request attribute key storing the resolved [`Locale`] (Symfony `_locale`).
pub const LOCALE_ATTRIBUTE: &str = "_locale";

/// Default cookie name for sticky locale choice.
pub const DEFAULT_LOCALE_COOKIE: &str = "_locale";

/// Default query parameter for explicit locale override.
pub const DEFAULT_LOCALE_QUERY: &str = "_locale";

/// Resolves a request locale from Symfony-shaped sources.
///
/// Priority (highest first):
/// 1. Existing [`LOCALE_ATTRIBUTE`] on the request
/// 2. Query parameter (default `_locale`)
/// 3. Cookie (default `_locale`)
/// 4. `Accept-Language` header against the allowed list
/// 5. Configured default locale
#[derive(Clone, Debug)]
pub struct LocaleNegotiator {
    default_locale: Locale,
    allowed: Vec<Locale>,
    cookie_name: String,
    query_param: String,
}

impl LocaleNegotiator {
    /// Negotiator with `default_locale` and matching allowed list.
    #[must_use]
    pub fn new(default_locale: Locale) -> Self {
        Self {
            allowed: vec![default_locale.clone()],
            default_locale,
            cookie_name: DEFAULT_LOCALE_COOKIE.into(),
            query_param: DEFAULT_LOCALE_QUERY.into(),
        }
    }

    /// Restricts negotiation to `allowed` (must include the default).
    #[must_use]
    pub fn with_allowed(mut self, allowed: Vec<Locale>) -> Self {
        if allowed.iter().all(|item| item != &self.default_locale) {
            let mut list = vec![self.default_locale.clone()];
            list.extend(allowed);
            self.allowed = list;
        } else {
            self.allowed = allowed;
        }
        self
    }

    /// Cookie name used for sticky locale.
    #[must_use]
    pub fn with_cookie_name(mut self, name: impl Into<String>) -> Self {
        self.cookie_name = name.into();
        self
    }

    /// Query parameter name for explicit override.
    #[must_use]
    pub fn with_query_param(mut self, name: impl Into<String>) -> Self {
        self.query_param = name.into();
        self
    }

    /// Default locale.
    #[must_use]
    pub const fn default_locale(&self) -> &Locale {
        &self.default_locale
    }

    /// Allowed locales.
    #[must_use]
    pub fn allowed(&self) -> &[Locale] {
        &self.allowed
    }

    /// Resolves locale and stores it on [`LOCALE_ATTRIBUTE`].
    pub fn apply(&self, request: &mut Request) -> Locale {
        let locale = self.resolve(request);
        request
            .attributes_mut()
            .insert(LOCALE_ATTRIBUTE, locale.clone());
        locale
    }

    /// Resolves without mutating attributes.
    #[must_use]
    pub fn resolve(&self, request: &Request) -> Locale {
        if let Some(existing) = request.attributes().get::<Locale>(LOCALE_ATTRIBUTE) {
            if self.is_allowed(existing) {
                return existing.clone();
            }
        }
        if let Some(from_query) = self.locale_from_query(request) {
            return from_query;
        }
        if let Some(from_cookie) = self.locale_from_cookie(request) {
            return from_cookie;
        }
        if let Some(from_header) = self.locale_from_accept_language(request) {
            return from_header;
        }
        self.default_locale.clone()
    }

    fn is_allowed(&self, locale: &Locale) -> bool {
        self.allowed.iter().any(|item| item == locale)
            || self
                .allowed
                .iter()
                .any(|item| item.language() == locale.language())
    }

    fn match_allowed(&self, candidate: &Locale) -> Option<Locale> {
        self.allowed
            .iter()
            .find(|item| *item == candidate)
            .cloned()
            .or_else(|| {
                self.allowed
                    .iter()
                    .find(|item| item.language() == candidate.language())
                    .cloned()
            })
    }

    fn locale_from_query(&self, request: &Request) -> Option<Locale> {
        let query = request.query().unwrap_or("");
        for pair in query.split('&') {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next().unwrap_or("");
            let value = parts.next().unwrap_or("");
            if key == self.query_param {
                let decoded = percent_decode(value);
                if let Ok(locale) = Locale::new(decoded) {
                    return self.match_allowed(&locale);
                }
            }
        }
        None
    }

    fn locale_from_cookie(&self, request: &Request) -> Option<Locale> {
        let header = request.headers().get("cookie")?;
        for part in header.split(';') {
            let part = part.trim();
            let Some((name, value)) = part.split_once('=') else {
                continue;
            };
            if name.trim() == self.cookie_name {
                if let Ok(locale) = Locale::new(value.trim()) {
                    return self.match_allowed(&locale);
                }
            }
        }
        None
    }

    fn locale_from_accept_language(&self, request: &Request) -> Option<Locale> {
        let header = request.headers().get("accept-language")?;
        let mut tags: Vec<(Locale, f32)> = Vec::new();
        for part in header.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let mut bits = part.split(';');
            let tag = bits.next().unwrap_or("").trim();
            if tag == "*" {
                continue;
            }
            let mut quality = 1.0_f32;
            for param in bits {
                let param = param.trim();
                if let Some(q) = param.strip_prefix("q=") {
                    quality = q.parse().unwrap_or(0.0);
                }
            }
            if let Ok(locale) = Locale::new(tag.replace('_', "-")) {
                tags.push((locale, quality));
            }
        }
        tags.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        for (locale, _) in tags {
            if let Some(matched) = self.match_allowed(&locale) {
                return Some(matched);
            }
        }
        None
    }
}

/// Reads the resolved locale from request attributes.
#[must_use]
pub fn request_locale(request: &Request) -> Option<&Locale> {
    request.attributes().get::<Locale>(LOCALE_ATTRIBUTE)
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hex = &input[i + 1..i + 3];
                if let Ok(value) = u8::from_str_radix(hex, 16) {
                    out.push(value);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            other => {
                out.push(other);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}
