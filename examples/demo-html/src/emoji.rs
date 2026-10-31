//! Curated emoji pack for the composer (`emojis` crate).

use std::fmt::Write as _;

use serenade_form::{escape_attr, escape_html};

/// Popular shortcodes used in the picker (GitHub-style names from the crate).
const PICKER_SHORTCODES: &[&str] = &[
    "grinning",
    "smiley",
    "smile",
    "joy",
    "rofl",
    "heart_eyes",
    "kissing_heart",
    "thinking",
    "sunglasses",
    "clap",
    "wave",
    "thumbsup",
    "thumbsdown",
    "fire",
    "sparkles",
    "star",
    "rocket",
    "tada",
    "heart",
    "blue_heart",
    "eyes",
    "100",
    "pray",
    "muscle",
    "coffee",
    "pizza",
    "musical_note",
    "movie_camera",
    "camera",
    "link",
    "bulb",
    "zap",
    "rainbow",
    "earth_africa",
    "crab",
    "owl",
];

/// HTML buttons for the emoji picker (wired by Clitorine).
#[must_use]
pub fn picker_html() -> String {
    let mut out = String::from(r#"<div class="emoji-picker" data-clitorine-emoji-panel hidden>"#);
    for name in PICKER_SHORTCODES {
        if let Some(emoji) = emojis::get_by_shortcode(name) {
            let glyph = emoji.as_str();
            let _ = write!(
                out,
                r#"<button type="button" class="emoji-btn" data-clitorine-emoji="{glyph}" title=":{name}:">{label}</button>"#,
                glyph = escape_attr(glyph),
                name = escape_attr(name),
                label = escape_html(glyph),
            );
        }
    }
    out.push_str("</div>");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picker_has_glyphs() {
        let html = picker_html();
        assert!(html.contains("emoji-btn"));
        assert!(html.contains("data-clitorine-emoji"));
        assert!(html.len() > 80);
    }
}
