//! HTML allowlist for Quill post bodies.

use std::collections::{HashMap, HashSet};

/// Sanitizes rich HTML from the composer before storage / render.
#[must_use]
pub fn sanitize_post_html(raw: &str) -> String {
    let mut tag_attributes = HashMap::new();
    tag_attributes.insert(
        "a",
        ["href", "title", "target"]
            .into_iter()
            .collect::<HashSet<_>>(),
    );
    ammonia::Builder::default()
        .tags(
            [
                "p",
                "br",
                "strong",
                "b",
                "em",
                "i",
                "u",
                "s",
                "blockquote",
                "ul",
                "ol",
                "li",
                "a",
                "span",
            ]
            .into_iter()
            .collect(),
        )
        .tag_attributes(tag_attributes)
        .url_schemes(["http", "https", "mailto"].into_iter().collect())
        .link_rel(Some("noopener noreferrer"))
        .clean(raw)
        .to_string()
}

/// Plain text length for the 2000-char demo cap.
#[must_use]
pub fn plain_len(html: &str) -> usize {
    ammonia::Builder::default()
        .tags(HashSet::new())
        .clean(html)
        .to_string()
        .chars()
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_script() {
        let out = sanitize_post_html(r"<p>hi</p><script>alert(1)</script>");
        assert!(out.contains("hi"));
        assert!(!out.contains("script"));
    }
}
