use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use scraper::{Html, Selector};
use url::Url;

const MAX_HTML_BYTES: usize = 512 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub struct LinkPreview {
    pub title: String,
    pub image_url: Option<String>,
}

pub fn web_url(raw: &str) -> Option<Url> {
    if raw.len() > 4096 || raw.chars().any(char::is_whitespace) {
        return None;
    }
    let url = Url::parse(raw).ok()?;
    (matches!(url.scheme(), "http" | "https")
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none())
    .then_some(url)
}

fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !(ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_broadcast()
                || ip.is_documentation()
                || a == 0
                || a >= 224
                || (a == 100 && (64..=127).contains(&b))
                || (a == 192 && b == 0 && c == 0)
                || (a == 192 && b == 88 && c == 99)
                || (a == 198 && matches!(b, 18 | 19)))
        }
        IpAddr::V6(ip) => {
            let segments = ip.segments();
            // Restrict IPv6 to global unicast, excluding transition/special ranges.
            segments[0] & 0xe000 == 0x2000
                && !(segments[0] == 0x2001 && (segments[1] <= 0x1ff || segments[1] == 0xdb8))
                && segments[0] != 0x2002
                && !(segments[0] == 0x3fff && segments[1] < 0x1000)
        }
    }
}

async fn fetch_page(mut url: Url) -> Result<(Url, String), String> {
    for _ in 0..=3 {
        let host = url.host_str().ok_or("Missing preview host")?;
        let port = url.port_or_known_default().ok_or("Missing preview port")?;
        if !matches!((url.scheme(), port), ("http", 80) | ("https", 443)) {
            return Err("Preview URLs must use standard HTTP(S) ports".into());
        }
        let addresses: Vec<SocketAddr> = match url.host() {
            Some(url::Host::Ipv4(ip)) => vec![SocketAddr::new(ip.into(), port)],
            Some(url::Host::Ipv6(ip)) => vec![SocketAddr::new(ip.into(), port)],
            _ => tokio::net::lookup_host((host, port))
                .await
                .map_err(|err| err.to_string())?
                .collect(),
        };
        if addresses.is_empty() || addresses.iter().any(|addr| !public_ip(addr.ip())) {
            return Err("Preview host must resolve only to public addresses".into());
        }
        // Pin validated DNS results and revalidate every redirect to prevent SSRF.
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .resolve_to_addrs(host, &addresses)
            .timeout(Duration::from_secs(5))
            .user_agent("MyFeed/1.0 (article preview)")
            .build()
            .map_err(|err| err.to_string())?;
        let mut response = client
            .get(url.clone())
            .send()
            .await
            .map_err(|err| err.to_string())?;
        if response.status().is_redirection() {
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .ok_or("Redirect without Location")?
                .to_str()
                .map_err(|err| err.to_string())?;
            let next = url.join(location).map_err(|err| err.to_string())?;
            url = web_url(next.as_str()).ok_or("Invalid preview redirect")?;
            continue;
        }
        response
            .error_for_status_ref()
            .map_err(|err| err.to_string())?;
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("");
        if !matches!(
            content_type.split(';').next().unwrap_or("").trim(),
            "text/html" | "application/xhtml+xml"
        ) {
            return Err("Preview response is not HTML".into());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|err| err.to_string())? {
            if bytes.len() + chunk.len() > MAX_HTML_BYTES {
                return Err("Preview HTML exceeds 512 KiB".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        return Ok((url, String::from_utf8_lossy(&bytes).into_owned()));
    }
    Err("Too many preview redirects".into())
}

pub async fn fetch(raw: &str) -> Result<LinkPreview, String> {
    let url = web_url(raw).ok_or("Invalid article URL")?;
    let (url, html) = tokio::time::timeout(Duration::from_secs(8), fetch_page(url))
        .await
        .map_err(|_| "Article preview timed out")??;
    Ok(parse(&url, &html))
}

fn parse(url: &Url, html: &str) -> LinkPreview {
    let document = Html::parse_document(html);
    let meta_selector = Selector::parse("meta").expect("meta selector");
    let metadata = |names: &[&str]| {
        names.iter().find_map(|name| {
            document.select(&meta_selector).find_map(|element| {
                let key = element
                    .value()
                    .attr("property")
                    .or_else(|| element.value().attr("name"))?;
                let value = element.value().attr("content")?.trim();
                (key.eq_ignore_ascii_case(name) && !value.is_empty()).then(|| value.to_owned())
            })
        })
    };
    let title = metadata(&["og:title", "twitter:title"])
        .or_else(|| {
            let selector = Selector::parse("title").expect("title selector");
            document
                .select(&selector)
                .next()
                .map(|title| title.text().collect::<String>())
        })
        .filter(|title| !title.trim().is_empty())
        .unwrap_or_else(|| url.host_str().unwrap_or("").to_owned())
        .chars()
        .take(300)
        .collect();
    let image_url = metadata(&["og:image", "twitter:image"])
        .and_then(|image| url.join(&image).ok())
        .filter(|image| web_url(image.as_str()).is_some())
        .map(String::from);
    LinkPreview { title, image_url }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_metadata_and_relative_images() {
        let url = Url::parse("https://www.phoronix.com/news/article").unwrap();
        let preview = parse(
            &url,
            r#"<title>Fallback</title>
            <meta content="A &amp; B" property="og:title">
            <meta name="twitter:image" content="/image.png">"#,
        );
        assert_eq!(preview.title, "A & B");
        assert_eq!(
            preview.image_url.as_deref(),
            Some("https://www.phoronix.com/image.png")
        );
        let preview = parse(
            &url,
            "<title>Plain article</title><meta property='og:image' content='javascript:alert(1)'>",
        );
        assert_eq!(preview.title, "Plain article");
        assert!(preview.image_url.is_none());
    }

    #[test]
    fn rejects_unsafe_urls_and_network_addresses() {
        for raw in [
            "javascript:alert(1)",
            "file:///etc/passwd",
            "https://user:pass@example.com",
            "https://",
            "https://example.com/\n",
        ] {
            assert!(web_url(raw).is_none(), "{raw}");
        }
        for ip in [
            "127.0.0.1",
            "10.0.0.1",
            "169.254.169.254",
            "192.168.1.1",
            "100.64.0.1",
            "0.0.0.0",
            "::1",
            "fc00::1",
            "fe80::1",
            "::ffff:127.0.0.1",
        ] {
            assert!(!public_ip(ip.parse().unwrap()), "{ip}");
        }
        assert!(public_ip("8.8.8.8".parse().unwrap()));
    }

    #[tokio::test]
    async fn refuses_private_fetches_and_nonstandard_ports() {
        for raw in [
            "http://127.0.0.1/",
            "http://169.254.169.254/",
            "http://[::1]/",
            "https://example.com:8443/",
        ] {
            assert!(fetch(raw).await.is_err(), "{raw}");
        }
    }
}
