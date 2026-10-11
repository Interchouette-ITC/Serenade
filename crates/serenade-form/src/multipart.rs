//! `multipart/form-data` body parser.

use std::collections::HashMap;
use std::hash::BuildHasher;

use crate::file::{FileStorage, UploadedFile};
use crate::kind::{DEFAULT_MAX_MULTIPART_BODY, FileOptions};
use crate::{FormError, parse_urlencoded_multi};

type PartHeaders = Vec<(String, String)>;

/// Parsed multipart form: text fields and file uploads.
#[derive(Debug, Default)]
pub struct MultipartData {
    /// Text field values (submission order preserved per key).
    pub fields: HashMap<String, Vec<String>>,
    /// File uploads (last part wins per field name).
    pub files: HashMap<String, UploadedFile>,
}

/// Extracts the `boundary` parameter from a `Content-Type` header value.
#[must_use]
pub fn multipart_boundary(content_type: &str) -> Option<&str> {
    let lower = content_type.to_ascii_lowercase();
    if !lower.starts_with("multipart/form-data") {
        return None;
    }
    for part in content_type.split(';').skip(1) {
        let part = part.trim();
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };
        if key.trim().eq_ignore_ascii_case("boundary") {
            return Some(value.trim().trim_matches('"'));
        }
    }
    None
}

/// Returns `true` when `content_type` is `multipart/form-data`.
#[must_use]
pub fn is_multipart(content_type: Option<&str>) -> bool {
    content_type.is_some_and(|value| {
        value
            .split(';')
            .next()
            .is_some_and(|mime| mime.trim().eq_ignore_ascii_case("multipart/form-data"))
    })
}

/// Parses a `multipart/form-data` body.
///
/// `file_options` maps field names to [`FileOptions`] used for size, MIME, and storage.
/// Unknown file parts use [`FileOptions::default`].
///
/// # Errors
///
/// Returns [`FormError::Multipart`] when the body exceeds `max_body`, the boundary is
/// missing/invalid, or a part cannot be decoded. Returns [`FormError::Upload`] when a
/// declared file field violates size or MIME constraints.
pub fn parse_multipart<S: BuildHasher>(
    body: &[u8],
    boundary: &str,
    file_options: &HashMap<String, FileOptions, S>,
    max_body: u64,
) -> Result<MultipartData, FormError> {
    if body.len() as u64 > max_body {
        return Err(FormError::Multipart(format!(
            "body exceeds max size of {max_body} bytes"
        )));
    }
    if boundary.is_empty() {
        return Err(FormError::Multipart(String::from("empty boundary")));
    }
    let delimiter = format!("--{boundary}").into_bytes();
    let parts = split_multipart_parts(body, &delimiter)?;
    let mut data = MultipartData::default();
    for part in parts {
        if part.is_empty() {
            continue;
        }
        let (headers, content) = split_part_headers(part)?;
        let disposition = header_value(&headers, "content-disposition").ok_or_else(|| {
            FormError::Multipart(String::from("part missing Content-Disposition"))
        })?;
        let name = disposition_param(disposition, "name")
            .ok_or_else(|| FormError::Multipart(String::from("part missing name parameter")))?;
        let filename = disposition_param(disposition, "filename");
        if let Some(filename) = filename {
            // Empty file inputs post `filename=""` with an empty body; treat as absent.
            if filename.is_empty() && content.is_empty() {
                continue;
            }
            let content_type = header_value(&headers, "content-type").map(str::to_owned);
            let options = file_options
                .get(name)
                .cloned()
                .unwrap_or_else(FileOptions::default);
            let upload = build_upload(filename, content_type, content, &options)?;
            data.files.insert(name.to_owned(), upload);
        } else {
            let text = std::str::from_utf8(content)
                .map_err(|_| FormError::Multipart(String::from("text part is not UTF-8")))?;
            data.fields
                .entry(name.to_owned())
                .or_default()
                .push(text.to_owned());
        }
    }
    Ok(data)
}

/// Parses urlencoded or multipart from a request body using `Content-Type`.
///
/// # Errors
///
/// Propagates [`parse_urlencoded_multi`] or [`parse_multipart`] failures. Missing
/// multipart boundary yields [`FormError::Multipart`].
pub fn parse_form_body<S: BuildHasher>(
    content_type: Option<&str>,
    body: &[u8],
    file_options: &HashMap<String, FileOptions, S>,
) -> Result<MultipartData, FormError> {
    if is_multipart(content_type) {
        let boundary = content_type
            .and_then(multipart_boundary)
            .ok_or_else(|| FormError::Multipart(String::from("missing multipart boundary")))?;
        parse_multipart(body, boundary, file_options, DEFAULT_MAX_MULTIPART_BODY)
    } else {
        let fields = parse_urlencoded_multi(body)?;
        Ok(MultipartData {
            fields,
            files: HashMap::new(),
        })
    }
}

fn build_upload(
    filename: &str,
    content_type: Option<String>,
    content: &[u8],
    options: &FileOptions,
) -> Result<UploadedFile, FormError> {
    if content.len() as u64 > options.max_size {
        return Err(FormError::Upload(format!(
            "file exceeds max size of {} bytes",
            options.max_size
        )));
    }
    if !options.mime_types.is_empty() {
        let mime = content_type.as_deref().unwrap_or("");
        let allowed = options
            .mime_types
            .iter()
            .any(|allowed| mime_matches(mime, allowed));
        if !allowed {
            return Err(FormError::Upload(format!(
                "content type {mime:?} is not allowed"
            )));
        }
    }
    let filename = if filename.is_empty() {
        None
    } else {
        Some(filename.to_owned())
    };
    match options.storage {
        FileStorage::Memory => Ok(UploadedFile::from_bytes(
            filename,
            content_type,
            content.to_vec(),
        )),
        FileStorage::TempFile => UploadedFile::from_tempfile(filename, content_type, content),
    }
}

fn mime_matches(actual: &str, allowed: &str) -> bool {
    let actual = actual.trim().to_ascii_lowercase();
    let allowed = allowed.trim().to_ascii_lowercase();
    if allowed.ends_with("/*") {
        let prefix = &allowed[..allowed.len() - 1];
        return actual.starts_with(prefix);
    }
    actual == allowed
}

fn split_multipart_parts<'a>(body: &'a [u8], delimiter: &[u8]) -> Result<Vec<&'a [u8]>, FormError> {
    let mut parts = Vec::new();
    let mut rest = body;
    if rest.starts_with(delimiter) {
        rest = &rest[delimiter.len()..];
        rest = strip_leading_crlf(rest);
    } else if let Some(pos) = find_bytes(rest, delimiter) {
        rest = &rest[pos + delimiter.len()..];
        rest = strip_leading_crlf(rest);
    } else {
        return Err(FormError::Multipart(String::from("boundary not found")));
    }
    loop {
        if rest.starts_with(b"--") {
            break;
        }
        let Some(pos) = find_bytes(rest, delimiter) else {
            return Err(FormError::Multipart(String::from(
                "unterminated multipart body",
            )));
        };
        let mut part = &rest[..pos];
        part = strip_trailing_crlf(part);
        parts.push(part);
        rest = &rest[pos + delimiter.len()..];
        rest = strip_leading_crlf(rest);
    }
    Ok(parts)
}

fn split_part_headers(part: &[u8]) -> Result<(PartHeaders, &[u8]), FormError> {
    let sep = find_header_body_sep(part).ok_or_else(|| {
        FormError::Multipart(String::from("multipart part missing header terminator"))
    })?;
    let header_bytes = &part[..sep.start];
    let content = &part[sep.end..];
    let header_text = std::str::from_utf8(header_bytes)
        .map_err(|_| FormError::Multipart(String::from("multipart headers are not UTF-8")))?;
    let mut headers = Vec::new();
    for line in header_text.split('\n') {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| FormError::Multipart(String::from("malformed multipart header line")))?;
        headers.push((name.trim().to_ascii_lowercase(), value.trim().to_owned()));
    }
    Ok((headers, content))
}

struct Span {
    start: usize,
    end: usize,
}

fn find_header_body_sep(part: &[u8]) -> Option<Span> {
    if let Some(pos) = find_bytes(part, b"\r\n\r\n") {
        return Some(Span {
            start: pos,
            end: pos + 4,
        });
    }
    find_bytes(part, b"\n\n").map(|pos| Span {
        start: pos,
        end: pos + 2,
    })
}

fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
}

fn disposition_param<'a>(disposition: &'a str, key: &str) -> Option<&'a str> {
    for part in disposition.split(';').skip(1) {
        let part = part.trim();
        let Some((name, value)) = part.split_once('=') else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case(key) {
            return Some(value.trim().trim_matches('"'));
        }
    }
    None
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn strip_leading_crlf(bytes: &[u8]) -> &[u8] {
    if bytes.starts_with(b"\r\n") {
        &bytes[2..]
    } else if bytes.starts_with(b"\n") {
        &bytes[1..]
    } else {
        bytes
    }
}

fn strip_trailing_crlf(bytes: &[u8]) -> &[u8] {
    if bytes.ends_with(b"\r\n") {
        &bytes[..bytes.len() - 2]
    } else if bytes.ends_with(b"\n") {
        &bytes[..bytes.len() - 1]
    } else {
        bytes
    }
}
