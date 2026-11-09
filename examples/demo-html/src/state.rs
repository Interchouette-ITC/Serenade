use std::sync::Arc;

use serenade_http::{Request, Response, UrlMatcher};
use serenade_notifier::NullTransport;
use serenade_profiler::ProfileStore;
use serenade_search::MemorySearchAdapter;
use serenade_security::HmacCsrfTokenManager;
use serenade_translation::{Locale, LocaleNegotiator, Translator};

use crate::html::redirect_with_cookie;
use crate::i18n::Ui;
use crate::store::FeedStore;

pub const DEFAULT_BIND: &str = "127.0.0.1:8080";
pub const DEFAULT_CSRF: &str = "myfeed-dev-csrf-secret-change-me!!";
pub const DEFAULT_ADMIN: &str = "myfeed-dev-admin";
pub const ADMIN_COOKIE: &str = "myfeed_admin";
pub const LOCALE_COOKIE: &str = "_locale";
pub const MAX_IMAGE_BYTES: usize = 1024 * 1024;
pub const MAX_IMAGE_DATA: usize = 1_398_200;
pub const MAX_BODY_CHARS: usize = 2000;

pub struct AppState {
    pub store: FeedStore,
    pub index: MemorySearchAdapter,
    pub csrf: HmacCsrfTokenManager,
    pub matcher: UrlMatcher,
    pub admin_token: String,
    pub translator: Translator,
    pub negotiator: LocaleNegotiator,
    pub profiler: Arc<ProfileStore>,
    /// Wave 29 notifier dogfood (Null discards after validate).
    pub notifier: NullTransport,
}

fn cookie_value(request: &Request, name: &str) -> Option<String> {
    let header = request.headers().get("cookie")?;
    for part in header.split(';') {
        let part = part.trim();
        if let Some((key, value)) = part.split_once('=')
            && key.trim() == name
        {
            return Some(value.trim().to_owned());
        }
    }
    None
}

pub fn is_admin(request: &Request, expected: &str) -> bool {
    if request.headers().get("Authorization").is_some_and(|value| {
        let key = value.strip_prefix("Bearer ").unwrap_or(value);
        key == expected
    }) {
        return true;
    }
    cookie_value(request, ADMIN_COOKIE).is_some_and(|value| value == expected)
}

pub fn ui_for<'a>(state: &'a AppState, request: &Request) -> Ui<'a> {
    let locale = serenade_translation::request_locale(request)
        .cloned()
        .unwrap_or_else(|| state.negotiator.resolve(request));
    state.translator.set_locale(locale.clone());
    Ui::new(&state.translator, locale)
}

pub fn handle_locale_set(state: &AppState, request: &Request) -> Response {
    let code = request
        .attributes()
        .get::<String>("code")
        .map_or("en", String::as_str);
    let locale = Locale::new(code).unwrap_or_else(|_| Locale::new("en").expect("en"));
    let matched = state
        .negotiator
        .allowed()
        .iter()
        .find(|item| item.language() == locale.language())
        .cloned()
        .unwrap_or_else(|| state.negotiator.default_locale().clone());
    let cookie = format!(
        "{LOCALE_COOKIE}={tag}; Path=/; SameSite=Lax; Max-Age=31536000",
        tag = matched.as_str()
    );
    redirect_with_cookie("/", &cookie)
}
