use crate::{
    MinifyError,
    javascript::{minify_script, script_mode},
};

/// Minifies HTML and eligible inline JavaScript while preserving hydration markers.
///
/// Inline classic and module scripts are parsed before minification. Data blocks,
/// external scripts, comments, and whitespace-sensitive HTML content are left to
/// the HTML-aware minifier.
///
/// # Errors
///
/// Returns an error when an inline JavaScript block is invalid or minification
/// emits invalid UTF-8.
pub fn minify_html(html: &str) -> Result<String, MinifyError> {
    let html = minify_inline_scripts(html)?;
    let mut config = simple_minify_html::Cfg::new();
    config.keep_closing_tags = true;
    config.keep_comments = true;
    config.keep_html_and_head_opening_tags = true;
    config.remove_bangs = false;
    config.remove_processing_instructions = false;

    decode_html_output(simple_minify_html::minify(html.as_bytes(), Some(config)))
}

fn decode_html_output(output: Vec<u8>) -> Result<String, MinifyError> {
    String::from_utf8(output).map_err(MinifyError::from)
}

fn minify_inline_scripts(html: &str) -> Result<String, MinifyError> {
    let mut output = String::with_capacity(html.len());
    let mut cursor = 0;

    while let Some(relative_start) = html[cursor..].find('<') {
        let start = cursor + relative_start;
        output.push_str(&html[cursor..start]);

        if html[start..].starts_with("<!--") {
            let Some(comment_end) = html[start + 4..].find("-->") else {
                output.push_str(&html[start..]);
                return Ok(output);
            };
            let end = start + 4 + comment_end + 3;
            output.push_str(&html[start..end]);
            cursor = end;
            continue;
        }

        let Some(tag_end) = tag_end(html, start) else {
            output.push_str(&html[start..]);
            return Ok(output);
        };
        let tag = &html[start..tag_end];
        let (closing, name_start, name_end) = tag_name(tag);
        if name_start == name_end {
            output.push('<');
            cursor = start + 1;
            continue;
        }
        let name = &tag[name_start..name_end];
        if !closing && name.eq_ignore_ascii_case("script") {
            let Some((close_start, close_end)) = closing_tag(html, tag_end, "script") else {
                output.push_str(&html[start..]);
                return Ok(output);
            };
            let script = &html[tag_end..close_start];
            output.push_str(tag);
            if let Some(mode) = script_mode(&tag[name_end..]) {
                output.push_str(&minify_script(script, mode)?);
            } else {
                output.push_str(script);
            }
            output.push_str(&html[close_start..close_end]);
            cursor = close_end;
        } else if !closing && name.eq_ignore_ascii_case("plaintext") {
            output.push_str(&html[start..]);
            return Ok(output);
        } else if !closing
            && matches!(
                name.to_ascii_lowercase().as_str(),
                "textarea"
                    | "title"
                    | "style"
                    | "xmp"
                    | "iframe"
                    | "noembed"
                    | "noframes"
                    | "noscript"
            )
        {
            let Some((_, close_end)) = closing_tag(html, tag_end, name) else {
                output.push_str(&html[start..]);
                return Ok(output);
            };
            output.push_str(&html[start..close_end]);
            cursor = close_end;
        } else {
            output.push_str(tag);
            cursor = tag_end;
        }
    }

    output.push_str(&html[cursor..]);
    Ok(output)
}

fn tag_end(html: &str, start: usize) -> Option<usize> {
    let bytes = html.as_bytes();
    let mut quote = None;
    for (offset, byte) in bytes.iter().enumerate().skip(start + 1) {
        match (quote, byte) {
            (Some(current), next) if current == *next => quote = None,
            (None, b'\'' | b'"') => quote = Some(*byte),
            (None, b'>') => return Some(offset + 1),
            _ => {}
        }
    }
    None
}

fn tag_name(tag: &str) -> (bool, usize, usize) {
    let bytes = tag.as_bytes();
    let mut start = 1;
    while bytes.get(start).is_some_and(u8::is_ascii_whitespace) {
        start += 1;
    }
    let closing = bytes.get(start) == Some(&b'/');
    if closing {
        start += 1;
    }
    while bytes.get(start).is_some_and(u8::is_ascii_whitespace) {
        start += 1;
    }
    let mut end = start;
    while bytes
        .get(end)
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b':'))
    {
        end += 1;
    }
    (closing, start, end)
}

fn closing_tag(html: &str, from: usize, name: &str) -> Option<(usize, usize)> {
    let needle = format!("</{name}");
    let mut cursor = from;
    while let Some(relative) = html[cursor..]
        .to_ascii_lowercase()
        .find(&needle.to_ascii_lowercase())
    {
        let start = cursor + relative;
        let following = html.as_bytes().get(start + needle.len()).copied();
        if following.is_some_and(|byte| byte.is_ascii_whitespace() || matches!(byte, b'>' | b'/')) {
            return tag_end(html, start).map(|end| (start, end));
        }
        cursor = start + needle.len();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_utf8_minifier_output() {
        assert!(matches!(
            decode_html_output(vec![0xff]),
            Err(MinifyError::InvalidHtmlOutput(_))
        ));
    }

    #[test]
    fn minifies_html_and_preserves_hydration_markers_and_text_spacing() {
        let source = "<!doctype html>\n<html><head></head><body><p>Meet<!> <span>Serenade</span>!</p><pre>one\n two</pre><!--hk_0--></body></html>";
        let output = minify_html(source).expect("valid HTML should minify");

        assert!(output.contains("Meet<!> <span>Serenade</span>!"));
        assert!(output.contains("<pre>one\n two</pre>"));
        assert!(output.contains("<!--hk_0-->"));
        assert!(output.starts_with("<!doctype html><html><head>"));
        assert!(output.ends_with("</body></html>"));
        assert!(output.len() < source.len());
    }

    #[test]
    fn preserves_non_tag_text_malformed_markup_and_comments() {
        for source in [
            "plain text without tags",
            "<",
            "<tag attr='unterminated>",
            "<!-- unterminated comment",
            "<div>text",
        ] {
            assert_eq!(
                minify_inline_scripts(source).expect("incomplete input is preserved"),
                source
            );
        }
        assert_eq!(
            minify_inline_scripts("<>").expect("empty tag is preserved"),
            "<>"
        );
        assert_eq!(
            minify_inline_scripts("<!-- valid --><div>text</div>")
                .expect("valid comment is preserved"),
            "<!-- valid --><div>text</div>"
        );
    }

    #[test]
    fn preserves_unclosed_script_plaintext_and_raw_text_elements() {
        for source in [
            "<script>const x = 1",
            "<plaintext><script>do not parse this",
            "<textarea><script>do not parse this</script></textarea>",
            "<title>do not parse <script>text</script></title>",
            "<style>do not parse <script>text</script></style>",
            "<xmp>do not parse <script>text</script></xmp>",
            "<iframe>do not parse <script>text</script></iframe>",
            "<noembed>do not parse <script>text</script></noembed>",
            "<noframes>do not parse <script>text</script></noframes>",
            "<noscript>do not parse <script>text</script></noscript>",
            "<textarea>unclosed",
        ] {
            assert_eq!(
                minify_inline_scripts(source).expect("raw text stays unchanged"),
                source
            );
        }
    }

    #[test]
    fn minifies_inline_scripts_and_leaves_external_and_data_scripts_unchanged() {
        let source = r#"<script type=module>const value = 1 + 2;</script><script src="/app.js">invalid {</script><script type="application/json">{ "a": 1 }</script>"#;
        let output = minify_html(source).expect("only inline JavaScript is parsed");
        assert!(output.contains("<script type=module>"));
        assert!(!output.contains("const value = 1 + 2;"));
        assert!(output.contains(r"<script src=/app.js>invalid {</script>"));
        assert!(output.contains(r#"<script type=application/json>{ "a": 1 }</script>"#));
    }

    #[test]
    fn rejects_invalid_inline_javascript() {
        assert!(matches!(
            minify_inline_scripts("<script>const = ;</script>"),
            Err(MinifyError::JavaScript(_))
        ));
    }

    #[test]
    fn finds_tag_ends_with_both_quote_styles_and_rejects_unterminated_quotes() {
        assert_eq!(tag_end("<a x='>'>", 0), Some(9));
        assert_eq!(tag_end("<a x=\">\">", 0), Some(9));
        assert_eq!(tag_end("<a x='>", 0), None);
    }

    #[test]
    fn parses_tag_names_with_closing_slashes_whitespace_and_namespaces() {
        assert_eq!(tag_name("< div>"), (false, 2, 5));
        assert_eq!(tag_name("</ div>"), (true, 3, 6));
        assert_eq!(tag_name("<x-widget:data>"), (false, 1, 14));
        assert_eq!(tag_name("<>"), (false, 1, 1));
    }

    #[test]
    fn closing_tag_skips_prefix_matches_and_rejects_incomplete_tags() {
        assert!(closing_tag("</scriptx></script>", 0, "script").is_some());
        assert!(closing_tag("</script >", 0, "script").is_some());
        assert!(closing_tag("</script/>", 0, "script").is_some());
        assert!(closing_tag("</script", 0, "script").is_none());
        assert_eq!(closing_tag("no close", 0, "script"), None);
        assert_eq!(closing_tag("</script x='", 0, "script"), None);
    }
}
