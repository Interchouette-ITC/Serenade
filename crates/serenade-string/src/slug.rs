//! URL-safe slug helper.

/// Builds a lowercase slug from `input` (letters, digits, hyphens).
///
/// Non-alphanumeric runs become a single `-`. Leading/trailing hyphens are stripped.
#[must_use]
pub fn slug(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev_hyphen = true;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_hyphen = false;
        } else if !prev_hyphen {
            out.push('-');
            prev_hyphen = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}
