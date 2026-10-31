//! Embed / media URL allowlist (`YouTube`, `SoundCloud`, direct image / mp4).

use serenade_form::escape_attr;

/// Returns `true` when `url` may be embedded or shown as media.
#[must_use]
pub fn is_allowed_embed(url: &str) -> bool {
    if is_allowed_image_or_video(url) {
        return true;
    }
    let Some(host) = host_of(url) else {
        return false;
    };
    let host = host.to_ascii_lowercase();
    matches!(
        host.as_str(),
        "youtube.com"
            | "www.youtube.com"
            | "m.youtube.com"
            | "youtu.be"
            | "youtube-nocookie.com"
            | "www.youtube-nocookie.com"
            | "soundcloud.com"
            | "www.soundcloud.com"
            | "w.soundcloud.com"
            | "on.soundcloud.com"
    )
}

/// Builds a safe media snippet, or `None` when the URL is not allowlisted.
#[must_use]
pub fn embed_html(url: &str) -> Option<String> {
    if !is_allowed_embed(url) {
        return None;
    }
    if let Some(id) = youtube_id(url) {
        let src = format!("https://www.youtube-nocookie.com/embed/{id}");
        return Some(format!(
            r#"<div class="embed ratio ratio-16x9"><iframe src="{src}" title="YouTube" loading="lazy" referrerpolicy="strict-origin-when-cross-origin" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture" allowfullscreen></iframe></div>"#,
            src = escape_attr(&src)
        ));
    }
    if is_soundcloud(url) {
        let src = format!(
            "https://w.soundcloud.com/player/?url={}&color=%230066cc&auto_play=false&hide_related=true&show_comments=false&show_user=true&show_reposts=false&show_teaser=false",
            urlencoding_lite(url)
        );
        return Some(format!(
            r#"<div class="embed"><iframe height="166" scrolling="no" frameborder="no" allow="autoplay" src="{src}" title="SoundCloud" loading="lazy"></iframe></div>"#,
            src = escape_attr(&src)
        ));
    }
    if looks_like_image(url) {
        return Some(format!(
            r#"<div class="embed-media"><img class="img-fluid rounded" src="{src}" alt="posted media" loading="lazy" /></div>"#,
            src = escape_attr(url)
        ));
    }
    if looks_like_mp4(url) {
        return Some(format!(
            r#"<div class="embed-media"><video class="w-100 rounded" controls preload="metadata" src="{src}"></video></div>"#,
            src = escape_attr(url)
        ));
    }
    None
}

fn is_allowed_image_or_video(url: &str) -> bool {
    looks_like_image(url) || looks_like_mp4(url)
}

fn looks_like_image(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    (lower.starts_with("https://") || lower.starts_with("http://"))
        && (lower.contains(".jpg")
            || lower.contains(".jpeg")
            || lower.contains(".png")
            || lower.contains(".webp")
            || lower.contains(".gif"))
}

fn looks_like_mp4(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    (lower.starts_with("https://") || lower.starts_with("http://")) && lower.contains(".mp4")
}

fn is_soundcloud(url: &str) -> bool {
    host_of(url).is_some_and(|host| {
        let host = host.to_ascii_lowercase();
        host == "soundcloud.com"
            || host == "www.soundcloud.com"
            || host == "w.soundcloud.com"
            || host == "on.soundcloud.com"
    })
}

fn youtube_id(url: &str) -> Option<String> {
    let host = host_of(url)?.to_ascii_lowercase();
    if host == "youtu.be" {
        let rest = url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))?;
        let path = rest.split_once('/')?.1;
        let id = path.split(['?', '#']).next()?.trim();
        return valid_yt_id(id);
    }
    if !(host.contains("youtube.com") || host.contains("youtube-nocookie.com")) {
        return None;
    }
    if let Some(q) = url.split_once('?').map(|(_, q)| q) {
        for pair in q.split('&') {
            if let Some(id) = pair.strip_prefix("v=") {
                return valid_yt_id(id.split('#').next().unwrap_or(id));
            }
        }
    }
    if let Some(idx) = url.find("/embed/") {
        let id = url[idx + 7..].split(['?', '#', '/']).next()?;
        return valid_yt_id(id);
    }
    if let Some(idx) = url.find("/shorts/") {
        let id = url[idx + 8..].split(['?', '#', '/']).next()?;
        return valid_yt_id(id);
    }
    None
}

fn valid_yt_id(id: &str) -> Option<String> {
    if id.len() >= 6
        && id.len() <= 20
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        Some(id.to_owned())
    } else {
        None
    }
}

fn host_of(url: &str) -> Option<&str> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let host = rest
        .split('/')
        .next()?
        .split('?')
        .next()?
        .split('#')
        .next()?;
    if host.is_empty() || host.contains('@') {
        return None;
    }
    Some(host)
}

/// Minimal URL-encode for `SoundCloud` `url=` query param.
fn urlencoding_lite(value: &str) -> String {
    let mut out = String::with_capacity(value.len() * 2);
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(byte));
            }
            _ => {
                let _ = std::fmt::Write::write_fmt(&mut out, format_args!("%{byte:02X}"));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_and_youtube_embed() {
        assert!(is_allowed_embed(
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
        ));
        let html = embed_html("https://www.youtube.com/watch?v=dQw4w9WgXcQ").expect("yt");
        assert!(html.contains("youtube-nocookie.com/embed/dQw4w9WgXcQ"));
        assert!(!html.contains("watch?v="));
        assert!(is_allowed_embed("https://soundcloud.com/artist/track"));
        assert!(is_allowed_embed("https://cdn.example.com/clip.mp4"));
        assert!(is_allowed_embed("https://cdn.example.com/pic.jpg"));
        assert!(!is_allowed_embed("https://evil.example/x"));
        assert!(!is_allowed_embed("javascript:alert(1)"));
        assert!(embed_html("https://youtu.be/dQw4w9WgXcQ").is_some());
        assert!(embed_html("https://evil.example").is_none());
    }
}
