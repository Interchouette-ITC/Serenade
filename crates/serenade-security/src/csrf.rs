//! CSRF token manager (Symfony `CsrfTokenManager` analogue).
//!
//! Tokens are HMAC-signed and **stateless**: validation recomputes the MAC.
//! Forms embed the value in `_token`; no server session store is required for v0.

use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

use crate::SecurityError;

type HmacSha256 = Hmac<Sha256>;

/// Default HTML form field name for the CSRF token (Symfony `_token`).
pub const CSRF_FIELD_NAME: &str = "_token";

/// Issued CSRF token: intention id + opaque value for the form field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsrfToken {
    id: String,
    value: String,
}

impl CsrfToken {
    /// Builds a token from an intention id and opaque value.
    #[must_use]
    pub fn new(id: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            value: value.into(),
        }
    }

    /// Intention / form id (Symfony token id).
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Opaque value placed in the CSRF form field.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// Issues and validates CSRF tokens.
pub trait CsrfTokenManager: Send + Sync {
    /// Creates a new token for `token_id` (usually the form name).
    ///
    /// # Errors
    ///
    /// Returns [`SecurityError::CsrfGeneration`] when the RNG fails.
    fn get_token(&self, token_id: &str) -> Result<CsrfToken, SecurityError>;

    /// Returns whether `token` matches a previously issued value for its id.
    fn is_token_valid(&self, token: &CsrfToken) -> bool;
}

/// HMAC-SHA256 CSRF manager. Secret must be app-owned and stable across requests.
#[derive(Debug, Clone)]
pub struct HmacCsrfTokenManager {
    secret: Vec<u8>,
}

impl HmacCsrfTokenManager {
    /// Creates a manager keyed by `secret` (use a long random app secret).
    #[must_use]
    pub fn new(secret: impl AsRef<[u8]>) -> Self {
        Self {
            secret: secret.as_ref().to_vec(),
        }
    }

    fn sign(&self, token_id: &str, nonce_hex: &str) -> Result<String, SecurityError> {
        let mut mac =
            HmacSha256::new_from_slice(&self.secret).map_err(|_| SecurityError::CsrfGeneration)?;
        mac.update(token_id.as_bytes());
        mac.update(b":");
        mac.update(nonce_hex.as_bytes());
        Ok(hex_encode(&mac.finalize().into_bytes()))
    }
}

impl CsrfTokenManager for HmacCsrfTokenManager {
    fn get_token(&self, token_id: &str) -> Result<CsrfToken, SecurityError> {
        let mut nonce = [0_u8; 16];
        getrandom::fill(&mut nonce).map_err(|_| SecurityError::CsrfGeneration)?;
        let nonce_hex = hex_encode(&nonce);
        let mac_hex = self.sign(token_id, &nonce_hex)?;
        Ok(CsrfToken::new(token_id, format!("{nonce_hex}.{mac_hex}")))
    }

    fn is_token_valid(&self, token: &CsrfToken) -> bool {
        let Some((nonce_hex, mac_hex)) = token.value().split_once('.') else {
            return false;
        };
        if nonce_hex.is_empty() || mac_hex.is_empty() {
            return false;
        }
        let Ok(expected) = self.sign(token.id(), nonce_hex) else {
            return false;
        };
        expected.as_bytes().ct_eq(mac_hex.as_bytes()).into()
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_roundtrip() {
        let mgr = HmacCsrfTokenManager::new(b"test-secret-key-32bytes-minimum!!");
        let token = mgr.get_token("comment").expect("token");
        assert_eq!(token.id(), "comment");
        assert!(mgr.is_token_valid(&token));
        let forged = CsrfToken::new("comment", "deadbeef.badmac");
        assert!(!mgr.is_token_valid(&forged));
        let wrong_id = CsrfToken::new("other", token.value());
        assert!(!mgr.is_token_valid(&wrong_id));
    }
}
