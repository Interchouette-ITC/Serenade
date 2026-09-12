//! Cookie-backed session id + [`SessionStore`](crate::SessionStore) lifecycle.

use std::sync::Arc;

use crate::{Session, SessionError, SessionStore};

/// Default session cookie name.
pub const DEFAULT_SESSION_COOKIE: &str = "SERENADE_SESSION";

/// Options for the session id cookie written on commit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CookieSessionOptions {
    name: String,
    path: String,
    http_only: bool,
    secure: bool,
    same_site: SameSite,
    max_age_secs: Option<u64>,
}

impl Default for CookieSessionOptions {
    fn default() -> Self {
        Self {
            name: DEFAULT_SESSION_COOKIE.to_owned(),
            path: "/".to_owned(),
            http_only: true,
            secure: false,
            same_site: SameSite::Lax,
            max_age_secs: None,
        }
    }
}

impl CookieSessionOptions {
    /// Cookie name (default [`DEFAULT_SESSION_COOKIE`]).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the cookie name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Sets the cookie path.
    #[must_use]
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    /// Sets the `HttpOnly` flag.
    #[must_use]
    pub const fn with_http_only(mut self, http_only: bool) -> Self {
        self.http_only = http_only;
        self
    }

    /// Sets the `Secure` flag.
    #[must_use]
    pub const fn with_secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }

    /// Sets `SameSite`.
    #[must_use]
    pub const fn with_same_site(mut self, same_site: SameSite) -> Self {
        self.same_site = same_site;
        self
    }

    /// Sets `Max-Age` in seconds (`None` = session cookie).
    #[must_use]
    pub const fn with_max_age_secs(mut self, max_age_secs: Option<u64>) -> Self {
        self.max_age_secs = max_age_secs;
        self
    }

    /// Cookie name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// `SameSite` attribute for the session cookie.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SameSite {
    /// `SameSite=Lax`.
    Lax,
    /// `SameSite=Strict`.
    Strict,
    /// `SameSite=None` (requires `Secure` in browsers).
    None,
}

impl SameSite {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Lax => "Lax",
            Self::Strict => "Strict",
            Self::None => "None",
        }
    }
}

/// Opens and commits sessions using a store plus a session-id cookie.
pub struct CookieSession {
    store: Arc<dyn SessionStore>,
    options: CookieSessionOptions,
}

impl CookieSession {
    /// Binds `store` with default cookie options.
    #[must_use]
    pub fn new(store: Arc<dyn SessionStore>) -> Self {
        Self {
            store,
            options: CookieSessionOptions::default(),
        }
    }

    /// Binds `store` with custom cookie options.
    #[must_use]
    pub fn with_options(store: Arc<dyn SessionStore>, options: CookieSessionOptions) -> Self {
        Self { store, options }
    }

    /// Cookie options.
    #[must_use]
    pub const fn options(&self) -> &CookieSessionOptions {
        &self.options
    }

    /// Loads or creates a session from an optional raw `Cookie` request header.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when id generation or the store fails.
    pub fn open(&self, cookie_header: Option<&str>) -> Result<Session, SessionError> {
        if let Some(id) =
            cookie_header.and_then(|header| parse_cookie_value(header, self.options.name()))
        {
            if let Some(attributes) = self.store.load(&id)? {
                return Ok(Session::existing(id, attributes));
            }
            // Unknown id: start a fresh bag under a new id (avoids fixation on stale cookies).
        }
        Ok(Session::new(generate_session_id()?))
    }

    /// Persists `session` and returns a `Set-Cookie` header value when needed.
    ///
    /// Returns `None` when the session was neither new, dirty, nor invalidated.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the store fails.
    pub fn commit(&self, session: &Session) -> Result<Option<String>, SessionError> {
        if session.is_invalidated() {
            self.store.delete(session.id())?;
            return Ok(Some(format_set_cookie(
                &self.options,
                session.id(),
                ClearCookie::Yes,
            )));
        }
        if !session.is_dirty() {
            return Ok(None);
        }
        self.store.save(session.id(), &session.take_for_save())?;
        Ok(Some(format_set_cookie(
            &self.options,
            session.id(),
            ClearCookie::No,
        )))
    }
}

#[derive(Clone, Copy)]
enum ClearCookie {
    Yes,
    No,
}

/// Generates a new opaque session id (32 random bytes, hex-encoded).
///
/// # Errors
///
/// Returns [`SessionError::Generation`] when the RNG fails.
pub fn generate_session_id() -> Result<String, SessionError> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| SessionError::Generation)?;
    Ok(hex_encode(&bytes))
}

/// Reads `name` from a raw `Cookie` header (`a=1; b=2`).
#[must_use]
pub fn parse_cookie_value(cookie_header: &str, name: &str) -> Option<String> {
    for part in cookie_header.split(';') {
        let part = part.trim();
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };
        if key.trim() == name {
            return Some(value.trim().to_owned());
        }
    }
    None
}

fn format_set_cookie(options: &CookieSessionOptions, id: &str, clear: ClearCookie) -> String {
    let mut out = format!("{}={}", options.name, id);
    out.push_str("; Path=");
    out.push_str(&options.path);
    match clear {
        ClearCookie::Yes => out.push_str("; Max-Age=0"),
        ClearCookie::No => {
            if let Some(max_age) = options.max_age_secs {
                out.push_str("; Max-Age=");
                out.push_str(&max_age.to_string());
            }
        }
    }
    if options.http_only {
        out.push_str("; HttpOnly");
    }
    if options.secure {
        out.push_str("; Secure");
    }
    out.push_str("; SameSite=");
    out.push_str(options.same_site.as_str());
    out
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[usize::from(byte >> 4)] as char);
        out.push(HEX[usize::from(byte & 0x0f)] as char);
    }
    out
}
