//! Flash bag (Symfony `FlashBag` analogue) stored inside a [`Session`](crate::Session).

use std::collections::HashMap;

use crate::Session;

/// Session attribute key for serialized flash messages.
pub const FLASH_SESSION_KEY: &str = "_serenade.flashes";

/// One-shot messages keyed by type (`success`, `error`, …).
///
/// Backed by [`Session`] attribute [`FLASH_SESSION_KEY`].
pub struct FlashBag<'a> {
    session: &'a mut Session,
}

impl Session {
    /// Borrows the flash bag stored in this session.
    pub const fn flash(&mut self) -> FlashBag<'_> {
        FlashBag { session: self }
    }
}

impl FlashBag<'_> {
    /// Appends `message` under `kind`.
    pub fn add(&mut self, kind: impl AsRef<str>, message: impl Into<String>) {
        let kind = kind.as_ref();
        if kind.is_empty() {
            return;
        }
        let mut map = load(self.session);
        map.entry(kind.to_owned()).or_default().push(message.into());
        save(self.session, &map);
    }

    /// Returns messages for `kind` without removing them.
    #[must_use]
    pub fn peek(&self, kind: &str) -> Vec<String> {
        load(self.session).get(kind).cloned().unwrap_or_default()
    }

    /// Returns all flashes without removing them.
    #[must_use]
    pub fn peek_all(&self) -> HashMap<String, Vec<String>> {
        load(self.session)
    }

    /// Returns messages for `kind` and removes that type from the bag.
    pub fn get(&mut self, kind: &str) -> Vec<String> {
        let mut map = load(self.session);
        let messages = map.remove(kind).unwrap_or_default();
        save(self.session, &map);
        messages
    }

    /// Returns every flash and clears the bag.
    pub fn all(&mut self) -> HashMap<String, Vec<String>> {
        let map = load(self.session);
        self.session.remove(FLASH_SESSION_KEY);
        map
    }

    /// Drops all flash messages.
    pub fn clear(&mut self) {
        self.session.remove(FLASH_SESSION_KEY);
    }

    /// Whether any flashes are stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        load(self.session).is_empty()
    }
}

fn load(session: &Session) -> HashMap<String, Vec<String>> {
    session
        .get(FLASH_SESSION_KEY)
        .map_or_else(HashMap::new, decode)
}

fn save(session: &mut Session, map: &HashMap<String, Vec<String>>) {
    if map.is_empty() {
        session.remove(FLASH_SESSION_KEY);
        return;
    }
    session.set(FLASH_SESSION_KEY, encode(map));
}

/// Line format: `kind|message` with `\`, `|`, and newlines escaped.
fn encode(map: &HashMap<String, Vec<String>>) -> String {
    let mut out = String::new();
    let mut entries: Vec<(&String, &Vec<String>)> = map.iter().collect();
    entries.sort_by_key(|(kind, _)| *kind);
    for (kind, messages) in entries {
        for message in messages {
            out.push_str(&escape(kind));
            out.push('|');
            out.push_str(&escape(message));
            out.push('\n');
        }
    }
    out
}

fn decode(raw: &str) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for line in raw.lines() {
        if line.is_empty() {
            continue;
        }
        let Some((kind_raw, message_raw)) = split_once_unescaped(line) else {
            continue;
        };
        let kind = unescape(kind_raw);
        let message = unescape(message_raw);
        if kind.is_empty() {
            continue;
        }
        map.entry(kind).or_default().push(message);
    }
    map
}

fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '|' => out.push_str("\\|"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out
}

fn unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('\\') | None => out.push('\\'),
                Some('|') => out.push('|'),
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn split_once_unescaped(line: &str) -> Option<(&str, &str)> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i = i.saturating_add(2);
            continue;
        }
        if bytes[i] == b'|' {
            return Some((&line[..i], &line[i + 1..]));
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{FLASH_SESSION_KEY, decode, encode, escape, unescape};
    use crate::Session;

    #[test]
    fn add_peek_get_consume_and_clear() {
        let mut session = Session::new("s");
        {
            let mut flash = session.flash();
            flash.add("success", "Saved");
            flash.add("success", "Also");
            flash.add("error", "Nope");
            assert_eq!(flash.peek("success"), vec!["Saved", "Also"]);
            assert!(!flash.is_empty());
            assert_eq!(flash.get("success"), vec!["Saved", "Also"]);
            assert_eq!(flash.peek("success"), Vec::<String>::new());
            assert_eq!(flash.peek("error"), vec!["Nope"]);
        }
        assert!(session.get(FLASH_SESSION_KEY).is_some());
        {
            let mut flash = session.flash();
            let all = flash.all();
            assert_eq!(all.get("error"), Some(&vec![String::from("Nope")]));
            assert!(flash.is_empty());
        }
        assert!(session.get(FLASH_SESSION_KEY).is_none());

        let mut session = Session::new("s2");
        session.flash().add("info", "hi");
        session.flash().clear();
        assert!(session.flash().is_empty());
    }

    #[test]
    fn empty_kind_ignored_and_roundtrip_escapes() {
        let mut session = Session::new("s");
        session.flash().add("", "x");
        assert!(session.flash().is_empty());

        session.flash().add("a|b", "line\nwith|pipe\\");
        let peeked = session.flash().peek("a|b");
        assert_eq!(peeked, vec!["line\nwith|pipe\\"]);

        let encoded = encode(&session.flash().peek_all());
        let decoded = decode(&encoded);
        assert_eq!(
            decoded.get("a|b"),
            Some(&vec![String::from("line\nwith|pipe\\")])
        );

        assert_eq!(unescape(&escape("a|b\\c\nd\r")), "a|b\\c\nd\r");
        assert!(decode("nonsplit").is_empty());
        assert!(decode("\n").is_empty());
        assert!(decode("|orphan").is_empty());
        assert_eq!(unescape("trail\\"), "trail\\");
        assert_eq!(unescape("\\z"), "\\z");
    }

    #[test]
    fn survives_session_attribute_reload() {
        let mut session = Session::new("s");
        session.flash().add("notice", "Hello");
        let attrs = session.attributes().clone();
        let mut reloaded = Session::existing("s", attrs);
        assert_eq!(reloaded.flash().get("notice"), vec!["Hello"]);
        assert!(reloaded.flash().is_empty());
    }
}
