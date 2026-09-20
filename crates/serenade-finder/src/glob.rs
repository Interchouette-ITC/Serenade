//! Simple `*` / `?` name matching (basename only).

/// Match `name` against a pattern with `*` (any run) and `?` (one char).
#[must_use]
pub const fn name_matches(pattern: &str, name: &str) -> bool {
    match_chars(pattern.as_bytes(), name.as_bytes())
}

const fn match_chars(pattern: &[u8], name: &[u8]) -> bool {
    let mut p = 0;
    let mut n = 0;
    let mut star_p: Option<usize> = None;
    let mut star_n = 0;
    while n < name.len() {
        if p < pattern.len() && (pattern[p] == b'?' || pattern[p] == name[n]) {
            p += 1;
            n += 1;
        } else if p < pattern.len() && pattern[p] == b'*' {
            star_p = Some(p);
            star_n = n;
            p += 1;
        } else if let Some(sp) = star_p {
            p = sp + 1;
            star_n += 1;
            n = star_n;
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == b'*' {
        p += 1;
    }
    p == pattern.len()
}

#[cfg(test)]
mod tests {
    use super::name_matches;

    #[test]
    fn glob_basics() {
        assert!(name_matches("*.rs", "lib.rs"));
        assert!(!name_matches("*.rs", "lib.toml"));
        assert!(name_matches("a?c", "abc"));
        assert!(!name_matches("a?c", "ac"));
        assert!(name_matches("*", "anything"));
        assert!(name_matches("pre*", "prefix"));
        assert!(!name_matches("pre*", "xprefix"));
    }
}
