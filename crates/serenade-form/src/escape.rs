//! HTML escaping (Twig autoescape analogue).

/// Escapes text for HTML body content (`&`, `<`, `>`, `"`, `'`).
#[must_use]
pub fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Escapes a value for use inside a double-quoted HTML attribute.
#[must_use]
pub fn escape_attr(input: &str) -> String {
    escape_html(input)
}
