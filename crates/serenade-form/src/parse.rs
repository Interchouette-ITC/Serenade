//! `application/x-www-form-urlencoded` body parser.

use std::collections::HashMap;

use crate::FormError;

/// Parses a URL-encoded form body into string fields (last value wins on duplicates).
///
/// # Errors
///
/// Returns [`FormError::InvalidEncoding`] when percent-decoding fails.
pub fn parse_urlencoded(body: &[u8]) -> Result<HashMap<String, String>, FormError> {
    let text = std::str::from_utf8(body).map_err(|_| FormError::InvalidEncoding)?;
    let mut map = HashMap::new();
    if text.is_empty() {
        return Ok(map);
    }
    for pair in text.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (raw_key, raw_value) = pair.split_once('=').unwrap_or((pair, ""));
        let key = percent_decode(raw_key)?;
        let value = percent_decode(raw_value)?;
        map.insert(key, value);
    }
    Ok(map)
}

fn percent_decode(input: &str) -> Result<String, FormError> {
    let mut out = Vec::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hi = from_hex(bytes[i + 1])?;
                let lo = from_hex(bytes[i + 2])?;
                out.push((hi << 4) | lo);
                i += 3;
            }
            b'%' => return Err(FormError::InvalidEncoding),
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).map_err(|_| FormError::InvalidEncoding)
}

const fn from_hex(byte: u8) -> Result<u8, FormError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(FormError::InvalidEncoding),
    }
}
