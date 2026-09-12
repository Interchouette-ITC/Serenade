//! Session bag, [`SessionStore`] contract, flash bag, in-memory store, and cookie session id.
//!
//! HTTP middleware lands in follow-up work. See `docs-dev/SESSION.md`.

mod cookie;
mod error;
mod flash;
mod memory;
mod session;
mod store;

pub use cookie::{
    CookieSession, CookieSessionOptions, DEFAULT_SESSION_COOKIE, SameSite, generate_session_id,
    parse_cookie_value,
};
pub use error::SessionError;
pub use flash::{FLASH_SESSION_KEY, FlashBag};
pub use memory::MemorySessionStore;
pub use session::Session;
pub use store::SessionStore;

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{
        CookieSession, CookieSessionOptions, DEFAULT_SESSION_COOKIE, MemorySessionStore, SameSite,
        Session, SessionError, SessionStore, generate_session_id, parse_cookie_value, version,
    };

    #[test]
    fn version_is_non_empty() {
        assert_ne!(version(), "");
    }

    #[test]
    fn bag_set_get_remove_clear() {
        let mut session = Session::new("abc");
        assert!(session.is_new());
        assert_eq!(session.id(), "abc");
        session.set("user", "42");
        assert_eq!(session.get("user"), Some("42"));
        assert!(!session.attributes().is_empty());
        assert_eq!(session.remove("user"), Some(String::from("42")));
        assert!(session.get("user").is_none());
        assert!(session.remove("missing").is_none());
        session.clear();
        session.set("a", "1");
        session.clear();
        assert!(session.get("a").is_none());
    }

    #[test]
    fn memory_store_roundtrip_and_delete() {
        let store = MemorySessionStore::new();
        let mut map = std::collections::HashMap::new();
        map.insert(String::from("k"), String::from("v"));
        store.save("s1", &map).expect("save");
        assert_eq!(
            store.load("s1").expect("load").as_ref().map(|m| m.get("k")),
            Some(Some(&String::from("v")))
        );
        store.delete("s1").expect("delete");
        assert!(store.load("s1").expect("load").is_none());
        assert!(matches!(
            store.load(""),
            Err(SessionError::InvalidId { .. })
        ));
    }

    #[test]
    fn cookie_session_open_commit_reload() {
        let store = Arc::new(MemorySessionStore::new());
        let cookies = CookieSession::with_options(
            store,
            CookieSessionOptions::new()
                .with_name("SID")
                .with_secure(true)
                .with_same_site(SameSite::Strict)
                .with_max_age_secs(Some(3600)),
        );
        let mut session = cookies.open(None).expect("open");
        assert!(session.is_new());
        session.set("cart", "1");
        let set_cookie = cookies.commit(&session).expect("commit").expect("header");
        assert!(set_cookie.starts_with("SID="));
        assert!(set_cookie.contains("HttpOnly"));
        assert!(set_cookie.contains("Secure"));
        assert!(set_cookie.contains("SameSite=Strict"));
        assert!(set_cookie.contains("Max-Age=3600"));

        let id = session.id().to_owned();
        let header = format!("SID={id}; other=x");
        let loaded = cookies.open(Some(&header)).expect("reopen");
        assert!(!loaded.is_new());
        assert_eq!(loaded.get("cart"), Some("1"));
        assert!(cookies.commit(&loaded).expect("noop").is_none());
    }

    #[test]
    fn invalidate_clears_store_and_cookie() {
        let store = Arc::new(MemorySessionStore::new());
        let cookies = CookieSession::new(Arc::clone(&store) as Arc<dyn SessionStore>);
        let mut session = cookies.open(None).expect("open");
        session.set("x", "1");
        cookies.commit(&session).expect("commit").expect("set");
        let header = format!("{DEFAULT_SESSION_COOKIE}={}", session.id());
        let mut again = cookies.open(Some(&header)).expect("load");
        assert_eq!(again.get("x"), Some("1"));
        again.invalidate();
        let clear = cookies.commit(&again).expect("clear").expect("expire");
        assert!(clear.contains("Max-Age=0"));
        assert!(store.load(session.id()).expect("load").is_none());
    }

    #[test]
    fn stale_cookie_gets_new_id() {
        let store = Arc::new(MemorySessionStore::new());
        let cookies = CookieSession::new(store);
        let session = cookies
            .open(Some(&format!("{DEFAULT_SESSION_COOKIE}=deadbeef")))
            .expect("open");
        assert!(session.is_new());
        assert_ne!(session.id(), "deadbeef");
    }

    #[test]
    fn parse_cookie_and_generate_id() {
        assert_eq!(
            parse_cookie_value("a=1; SERENADE_SESSION=abc; b=2", DEFAULT_SESSION_COOKIE),
            Some(String::from("abc"))
        );
        assert!(parse_cookie_value("a=1", DEFAULT_SESSION_COOKIE).is_none());
        assert!(parse_cookie_value("lonely; SERENADE_SESSION=z", DEFAULT_SESSION_COOKIE).is_some());
        let id = generate_session_id().expect("rng");
        assert_eq!(id.len(), 64);
    }

    #[test]
    fn options_accessor() {
        let store = Arc::new(MemorySessionStore::new());
        let cookies =
            CookieSession::with_options(store, CookieSessionOptions::new().with_name("X"));
        assert_eq!(cookies.options().name(), "X");
    }

    #[test]
    fn same_site_none_and_path() {
        let store = Arc::new(MemorySessionStore::new());
        let cookies = CookieSession::with_options(
            store,
            CookieSessionOptions::new()
                .with_path("/app")
                .with_http_only(false)
                .with_same_site(SameSite::None)
                .with_secure(true),
        );
        let mut session = cookies.open(None).expect("open");
        session.set("k", "v");
        let header = cookies.commit(&session).expect("commit").expect("set");
        assert!(header.contains("Path=/app"));
        assert!(!header.contains("HttpOnly"));
        assert!(header.contains("SameSite=None"));
    }
}
