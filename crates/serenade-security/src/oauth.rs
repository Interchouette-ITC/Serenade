//! OAuth 2.0 / OIDC relying-party helpers (feature `oauth`).
//!
//! Builds authorization URLs with PKCE, shapes token-endpoint form bodies, and
//! maps an `IdP` subject into [`UsernamePasswordToken`]. This is **not** an
//! authorization server and does **not** verify JWT signatures.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::{InMemoryUser, SecurityError, UsernamePasswordToken};

/// OAuth 2.0 / OIDC client (relying party) settings.
#[derive(Debug, Clone)]
pub struct OAuthClientConfig {
    client_id: String,
    client_secret: Option<String>,
    authorization_endpoint: String,
    token_endpoint: String,
    redirect_uri: String,
    scopes: Vec<String>,
}

impl OAuthClientConfig {
    /// Creates a client config. `client_secret` is optional (public clients / PKCE).
    #[must_use]
    pub fn new(
        client_id: impl Into<String>,
        authorization_endpoint: impl Into<String>,
        token_endpoint: impl Into<String>,
        redirect_uri: impl Into<String>,
    ) -> Self {
        Self {
            client_id: client_id.into(),
            client_secret: None,
            authorization_endpoint: authorization_endpoint.into(),
            token_endpoint: token_endpoint.into(),
            redirect_uri: redirect_uri.into(),
            scopes: Vec::new(),
        }
    }

    /// Sets the confidential-client secret (omit for public PKCE clients).
    #[must_use]
    pub fn with_client_secret(mut self, secret: impl Into<String>) -> Self {
        self.client_secret = Some(secret.into());
        self
    }

    /// Sets space-joined OAuth scopes (for example `openid profile email`).
    #[must_use]
    pub fn with_scopes(mut self, scopes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.scopes = scopes.into_iter().map(Into::into).collect();
        self
    }

    /// OAuth `client_id`.
    #[must_use]
    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    /// Optional client secret.
    #[must_use]
    pub fn client_secret(&self) -> Option<&str> {
        self.client_secret.as_deref()
    }

    /// Authorization endpoint URL.
    #[must_use]
    pub fn authorization_endpoint(&self) -> &str {
        &self.authorization_endpoint
    }

    /// Token endpoint URL.
    #[must_use]
    pub fn token_endpoint(&self) -> &str {
        &self.token_endpoint
    }

    /// Registered redirect URI.
    #[must_use]
    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }

    /// Configured scopes.
    #[must_use]
    pub fn scopes(&self) -> &[String] {
        &self.scopes
    }
}

/// Authorization redirect URL plus CSRF `state` and PKCE `code_verifier`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizationRequest {
    url: String,
    state: String,
    code_verifier: String,
}

impl AuthorizationRequest {
    /// Full redirect URL for the user-agent.
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Opaque `state` value to store and compare on callback.
    #[must_use]
    pub fn state(&self) -> &str {
        &self.state
    }

    /// PKCE verifier to send on the token exchange.
    #[must_use]
    pub fn code_verifier(&self) -> &str {
        &self.code_verifier
    }
}

/// Builds an authorization request with a fresh random `state` and PKCE verifier.
///
/// # Errors
///
/// Returns [`SecurityError::OAuth`] when the RNG fails.
pub fn build_authorization_request(
    config: &OAuthClientConfig,
) -> Result<AuthorizationRequest, SecurityError> {
    let state = random_url_safe(32)?;
    let code_verifier = random_url_safe(32)?;
    Ok(build_authorization_request_with(
        config,
        state,
        code_verifier,
    ))
}

/// Builds an authorization request with caller-supplied `state` and PKCE verifier.
#[must_use]
pub fn build_authorization_request_with(
    config: &OAuthClientConfig,
    state: impl Into<String>,
    code_verifier: impl Into<String>,
) -> AuthorizationRequest {
    let state = state.into();
    let code_verifier = code_verifier.into();
    let challenge = pkce_s256_challenge(&code_verifier);
    let mut url = String::new();
    url.push_str(config.authorization_endpoint());
    url.push(if config.authorization_endpoint().contains('?') {
        '&'
    } else {
        '?'
    });
    push_query(&mut url, "response_type", "code");
    url.push('&');
    push_query(&mut url, "client_id", config.client_id());
    url.push('&');
    push_query(&mut url, "redirect_uri", config.redirect_uri());
    url.push('&');
    push_query(&mut url, "state", &state);
    url.push('&');
    push_query(&mut url, "code_challenge", &challenge);
    url.push('&');
    push_query(&mut url, "code_challenge_method", "S256");
    if !config.scopes().is_empty() {
        url.push('&');
        push_query(&mut url, "scope", &config.scopes().join(" "));
    }
    AuthorizationRequest {
        url,
        state,
        code_verifier,
    }
}

/// `application/x-www-form-urlencoded` body for the authorization-code token exchange.
#[must_use]
pub fn token_exchange_form(config: &OAuthClientConfig, code: &str, code_verifier: &str) -> String {
    let mut body = String::new();
    push_query(&mut body, "grant_type", "authorization_code");
    body.push('&');
    push_query(&mut body, "code", code);
    body.push('&');
    push_query(&mut body, "redirect_uri", config.redirect_uri());
    body.push('&');
    push_query(&mut body, "client_id", config.client_id());
    body.push('&');
    push_query(&mut body, "code_verifier", code_verifier);
    if let Some(secret) = config.client_secret() {
        body.push('&');
        push_query(&mut body, "client_secret", secret);
    }
    body
}

/// Parsed token-endpoint JSON body.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct OAuthTokenResponse {
    /// Access token string.
    pub access_token: String,
    /// Token type (usually `Bearer`).
    #[serde(default)]
    pub token_type: Option<String>,
    /// Lifetime in seconds when present.
    #[serde(default)]
    pub expires_in: Option<u64>,
    /// Refresh token when present.
    #[serde(default)]
    pub refresh_token: Option<String>,
    /// OIDC ID token (JWT) when present.
    #[serde(default)]
    pub id_token: Option<String>,
    /// Granted scope string when present.
    #[serde(default)]
    pub scope: Option<String>,
}

/// Parses a token-endpoint JSON response body.
///
/// # Errors
///
/// Returns [`SecurityError::OAuth`] when JSON is invalid or `access_token` is missing.
pub fn parse_token_response(body: &str) -> Result<OAuthTokenResponse, SecurityError> {
    serde_json::from_str(body).map_err(|err| SecurityError::OAuth {
        message: format!("invalid token response: {err}"),
    })
}

/// Sync token exchange. Apps implement this with their HTTP stack.
pub trait TokenExchanger: Send + Sync {
    /// POSTs the authorization code to the token endpoint.
    ///
    /// # Errors
    ///
    /// Returns [`SecurityError::OAuth`] (or [`SecurityError::Authentication`]) on failure.
    fn exchange_code(
        &self,
        config: &OAuthClientConfig,
        code: &str,
        code_verifier: &str,
    ) -> Result<OAuthTokenResponse, SecurityError>;
}

/// Fixed-response exchanger for tests.
#[derive(Debug, Clone)]
pub struct MockTokenExchanger {
    response: OAuthTokenResponse,
}

impl MockTokenExchanger {
    /// Returns `response` for every exchange.
    #[must_use]
    pub const fn new(response: OAuthTokenResponse) -> Self {
        Self { response }
    }
}

impl TokenExchanger for MockTokenExchanger {
    fn exchange_code(
        &self,
        _config: &OAuthClientConfig,
        _code: &str,
        _code_verifier: &str,
    ) -> Result<OAuthTokenResponse, SecurityError> {
        Ok(self.response.clone())
    }
}

/// Builds an authenticated security token from an `IdP` subject and roles.
#[must_use]
pub fn token_from_oidc_subject(
    subject: impl Into<String>,
    roles: impl IntoIterator<Item = impl Into<String>>,
    access_token: impl Into<String>,
) -> UsernamePasswordToken {
    let roles: Vec<String> = roles.into_iter().map(Into::into).collect();
    UsernamePasswordToken::authenticated(InMemoryUser::new(subject, roles), access_token)
}

/// Decodes the JWT payload segment **without** verifying the signature.
///
/// Production apps must verify the ID token with the `IdP` JWKS before trusting claims.
///
/// # Errors
///
/// Returns [`SecurityError::OAuth`] when the JWT shape or Base64/JSON is invalid.
pub fn decode_jwt_payload_json(jwt: &str) -> Result<serde_json::Value, SecurityError> {
    let payload_b64 = jwt.split('.').nth(1).ok_or_else(|| SecurityError::OAuth {
        message: "id_token is not a JWT".to_owned(),
    })?;
    let bytes = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|err| SecurityError::OAuth {
            message: format!("invalid JWT payload encoding: {err}"),
        })?;
    serde_json::from_slice(&bytes).map_err(|err| SecurityError::OAuth {
        message: format!("invalid JWT payload JSON: {err}"),
    })
}

/// Reads the OIDC `sub` claim from an unverified ID token payload.
///
/// # Errors
///
/// Returns [`SecurityError::OAuth`] when decode fails or `sub` is missing.
pub fn subject_from_id_token(id_token: &str) -> Result<String, SecurityError> {
    let payload = decode_jwt_payload_json(id_token)?;
    payload
        .get("sub")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| SecurityError::OAuth {
            message: "id_token payload missing sub".to_owned(),
        })
}

fn pkce_s256_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

fn random_url_safe(byte_len: usize) -> Result<String, SecurityError> {
    let mut buf = vec![0_u8; byte_len];
    getrandom::fill(&mut buf).map_err(|_| SecurityError::OAuth {
        message: "RNG failed".to_owned(),
    })?;
    Ok(URL_SAFE_NO_PAD.encode(buf))
}

fn push_query(out: &mut String, key: &str, value: &str) {
    out.push_str(key);
    out.push('=');
    percent_encode_into(out, value);
}

fn percent_encode_into(out: &mut String, value: &str) {
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(byte));
            }
            _ => {
                use std::fmt::Write as _;
                let _ = write!(out, "%{byte:02X}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> OAuthClientConfig {
        OAuthClientConfig::new(
            "client-1",
            "https://idp.example/authorize",
            "https://idp.example/token",
            "https://app.example/callback",
        )
        .with_scopes(["openid", "profile"])
        .with_client_secret("sekrit")
    }

    #[test]
    fn authorization_url_includes_pkce_and_state() {
        let req = build_authorization_request_with(
            &sample_config(),
            "st",
            "verifier-value-32chars________",
        );
        assert!(req.url().starts_with("https://idp.example/authorize?"));
        assert!(req.url().contains("response_type=code"));
        assert!(req.url().contains("client_id=client-1"));
        assert!(req.url().contains("state=st"));
        assert!(req.url().contains("code_challenge_method=S256"));
        assert!(req.url().contains("code_challenge="));
        assert!(req.url().contains("scope=openid%20profile"));
        assert_eq!(req.state(), "st");
        assert_eq!(req.code_verifier(), "verifier-value-32chars________");
    }

    #[test]
    fn authorization_url_appends_when_endpoint_has_query() {
        let config = OAuthClientConfig::new(
            "c",
            "https://idp.example/authorize?foo=1",
            "https://idp.example/token",
            "https://app.example/cb",
        );
        let req = build_authorization_request_with(&config, "s", "v");
        assert!(
            req.url()
                .starts_with("https://idp.example/authorize?foo=1&")
        );
    }

    #[test]
    fn token_form_includes_secret_and_pkce() {
        let body = token_exchange_form(&sample_config(), "auth-code", "verifier");
        assert!(body.contains("grant_type=authorization_code"));
        assert!(body.contains("code=auth-code"));
        assert!(body.contains("code_verifier=verifier"));
        assert!(body.contains("client_secret=sekrit"));
        assert!(body.contains("redirect_uri=https%3A%2F%2Fapp.example%2Fcallback"));
    }

    #[test]
    fn parse_token_and_mock_exchange() {
        let json = r#"{"access_token":"at","token_type":"Bearer","id_token":"x.eyJzdWIiOiJ1MSJ9.y","expires_in":3600}"#;
        let parsed = parse_token_response(json).expect("parse");
        assert_eq!(parsed.access_token, "at");
        assert_eq!(parsed.token_type.as_deref(), Some("Bearer"));
        let exchanger = MockTokenExchanger::new(parsed.clone());
        let got = exchanger
            .exchange_code(&sample_config(), "c", "v")
            .expect("exchange");
        assert_eq!(got, parsed);
    }

    #[test]
    fn parse_token_rejects_garbage() {
        let err = parse_token_response("not-json").expect_err("bad");
        assert!(matches!(err, SecurityError::OAuth { .. }));
    }

    #[test]
    fn subject_from_minimal_jwt() {
        // {"sub":"user-42"}
        let payload = URL_SAFE_NO_PAD.encode(br#"{"sub":"user-42"}"#);
        let jwt = format!("hdr.{payload}.sig");
        assert_eq!(subject_from_id_token(&jwt).expect("sub"), "user-42");
    }

    #[test]
    fn subject_rejects_non_jwt() {
        let err = subject_from_id_token("no-dots").expect_err("bad");
        assert!(matches!(err, SecurityError::OAuth { .. }));
    }

    #[test]
    fn token_from_subject_is_authenticated() {
        use crate::user::{TokenInterface, UserInterface};

        let token = token_from_oidc_subject("alice", ["ROLE_USER"], "access");
        assert!(token.is_authenticated());
        assert_eq!(
            token.user().map(UserInterface::user_identifier),
            Some("alice")
        );
        assert_eq!(token.credentials(), Some("access"));
    }

    #[test]
    fn build_authorization_request_uses_rng() {
        let a = build_authorization_request(&sample_config()).expect("a");
        let b = build_authorization_request(&sample_config()).expect("b");
        assert_ne!(a.state(), b.state());
        assert_ne!(a.code_verifier(), b.code_verifier());
        assert_ne!(a.state(), "");
        assert_ne!(a.code_verifier(), "");
    }

    #[test]
    fn config_accessors() {
        let config = sample_config();
        assert_eq!(config.client_id(), "client-1");
        assert_eq!(config.client_secret(), Some("sekrit"));
        assert_eq!(
            config.authorization_endpoint(),
            "https://idp.example/authorize"
        );
        assert_eq!(config.token_endpoint(), "https://idp.example/token");
        assert_eq!(config.redirect_uri(), "https://app.example/callback");
        assert_eq!(
            config.scopes(),
            &["openid".to_owned(), "profile".to_owned()]
        );
    }

    #[test]
    fn subject_rejects_missing_sub() {
        let payload = URL_SAFE_NO_PAD.encode(br#"{"aud":"x"}"#);
        let jwt = format!("hdr.{payload}.sig");
        let err = subject_from_id_token(&jwt).expect_err("no sub");
        assert!(matches!(err, SecurityError::OAuth { .. }));
    }
}
