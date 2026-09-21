//! ESP HTTP mail transport (feature `esp`).
//!
//! Shapes a SendGrid-class JSON mail API and a sync HTTP poster trait so apps
//! can plug `ureq`, `reqwest` blocking, or another client without Serenade
//! owning a vendor SDK. This is **not** an ESP provider.

use std::sync::{Arc, Mutex};

use serde_json::json;

use crate::null::validate_for_send;
use crate::{Address, Email, MailerError, Transport};

/// Authorization scheme for the ESP HTTP API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EspAuthScheme {
    /// `Authorization: Bearer <api_key>`.
    Bearer,
}

/// Endpoint and credentials for an ESP HTTP mail API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EspApiConfig {
    endpoint_url: String,
    api_key: String,
    auth_scheme: EspAuthScheme,
}

impl EspApiConfig {
    /// Creates config for `endpoint_url` authenticated with `api_key`.
    #[must_use]
    pub fn new(endpoint_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            endpoint_url: endpoint_url.into(),
            api_key: api_key.into(),
            auth_scheme: EspAuthScheme::Bearer,
        }
    }

    /// Sets the authorization scheme (default [`EspAuthScheme::Bearer`]).
    #[must_use]
    pub const fn auth_scheme(mut self, scheme: EspAuthScheme) -> Self {
        self.auth_scheme = scheme;
        self
    }

    /// Mail send endpoint URL.
    #[must_use]
    pub fn endpoint_url(&self) -> &str {
        &self.endpoint_url
    }

    /// API key / token value.
    #[must_use]
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    /// Authorization scheme.
    #[must_use]
    pub const fn auth_scheme_value(&self) -> EspAuthScheme {
        self.auth_scheme
    }

    /// Builds the `Authorization` header value for this config.
    #[must_use]
    pub fn authorization_header(&self) -> String {
        match self.auth_scheme {
            EspAuthScheme::Bearer => format!("Bearer {}", self.api_key),
        }
    }
}

/// HTTP response returned by an [`EspHttpPoster`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EspHttpResponse {
    status: u16,
    body: String,
}

impl EspHttpResponse {
    /// Creates a response with HTTP `status` and optional `body`.
    #[must_use]
    pub fn new(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            body: body.into(),
        }
    }

    /// HTTP status code.
    #[must_use]
    pub const fn status(&self) -> u16 {
        self.status
    }

    /// Response body text.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }

    /// Whether the status is in the 2xx success range.
    #[must_use]
    pub const fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }
}

/// Sync HTTP POST used by [`EspHttpTransport`]. Apps implement this.
pub trait EspHttpPoster: Send + Sync {
    /// POSTs `body` to `url` with the given header pairs.
    ///
    /// # Errors
    ///
    /// Returns [`MailerError::Transport`] when the HTTP call fails.
    fn post_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> Result<EspHttpResponse, MailerError>;
}

/// Captured request for [`MockEspHttpPoster`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockEspRequest {
    /// Request URL.
    pub url: String,
    /// Header pairs as owned strings.
    pub headers: Vec<(String, String)>,
    /// Raw JSON body bytes.
    pub body: Vec<u8>,
}

/// Scripted poster for unit tests.
#[derive(Debug, Clone)]
pub struct MockEspHttpPoster {
    response: EspHttpResponse,
    last: Arc<Mutex<Option<MockEspRequest>>>,
}

impl MockEspHttpPoster {
    /// Always returns `response`.
    #[must_use]
    pub fn new(response: EspHttpResponse) -> Self {
        Self {
            response,
            last: Arc::new(Mutex::new(None)),
        }
    }

    /// Always returns HTTP 202 with an empty body.
    #[must_use]
    pub fn accepted() -> Self {
        Self::new(EspHttpResponse::new(202, ""))
    }

    /// Last request observed by this mock, when any.
    #[must_use]
    pub fn last_request(&self) -> Option<MockEspRequest> {
        self.last
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl EspHttpPoster for MockEspHttpPoster {
    fn post_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> Result<EspHttpResponse, MailerError> {
        let request = MockEspRequest {
            url: url.to_owned(),
            headers: headers
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                .collect(),
            body: body.to_vec(),
        };
        *self
            .last
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(request);
        Ok(self.response.clone())
    }
}

/// HTTP ESP transport (SendGrid-class JSON mail API).
#[derive(Clone, Debug)]
pub struct EspHttpTransport<P> {
    config: EspApiConfig,
    poster: P,
}

impl<P> EspHttpTransport<P> {
    /// Creates a transport with `config` and sync HTTP `poster`.
    #[must_use]
    pub const fn new(config: EspApiConfig, poster: P) -> Self {
        Self { config, poster }
    }

    /// ESP API config.
    #[must_use]
    pub const fn config(&self) -> &EspApiConfig {
        &self.config
    }

    /// Underlying HTTP poster.
    #[must_use]
    pub const fn poster(&self) -> &P {
        &self.poster
    }
}

impl<P: EspHttpPoster> Transport for EspHttpTransport<P> {
    fn send(&self, email: &Email) -> Result<(), MailerError> {
        validate_for_send(email)?;
        let payload = build_esp_payload(email)?;
        let auth = self.config.authorization_header();
        let headers = [
            ("Authorization", auth.as_str()),
            ("Content-Type", "application/json"),
        ];
        let response = self
            .poster
            .post_json(self.config.endpoint_url(), &headers, &payload)?;
        if response.is_success() {
            return Ok(());
        }
        Err(MailerError::Transport {
            message: format!(
                "ESP HTTP {} {}",
                response.status(),
                truncate_body(response.body())
            ),
        })
    }
}

/// Builds a SendGrid-class JSON body from `email`.
///
/// # Errors
///
/// Returns [`MailerError::MissingSender`], [`MailerError::MissingRecipient`], or
/// [`MailerError::Transport`] when the message has no text/HTML body or JSON
/// encoding fails.
pub fn build_esp_payload(email: &Email) -> Result<Vec<u8>, MailerError> {
    validate_for_send(email)?;
    let from = email
        .from_addresses()
        .first()
        .ok_or(MailerError::MissingSender)?;
    let mut content = Vec::new();
    if let Some(text) = email.text_part() {
        content.push(json!({ "type": "text/plain", "value": text }));
    }
    if let Some(html) = email.html_part() {
        content.push(json!({ "type": "text/html", "value": html }));
    }
    if content.is_empty() {
        return Err(MailerError::Transport {
            message: "ESP payload requires text or HTML body".to_owned(),
        });
    }

    let mut personalization = json!({
        "to": addresses_json(email.to_addresses()),
    });
    if !email.cc_addresses().is_empty() {
        personalization["cc"] = addresses_json(email.cc_addresses());
    }
    if !email.bcc_addresses().is_empty() {
        personalization["bcc"] = addresses_json(email.bcc_addresses());
    }

    let mut payload = json!({
        "personalizations": [personalization],
        "from": address_json(from),
        "subject": email.subject_line(),
        "content": content,
    });
    if let Some(reply) = email.reply_to_addresses().first() {
        payload["reply_to"] = address_json(reply);
    }

    serde_json::to_vec(&payload).map_err(|error| MailerError::Transport {
        message: format!("ESP JSON encode: {error}"),
    })
}

fn addresses_json(addresses: &[Address]) -> serde_json::Value {
    serde_json::Value::Array(addresses.iter().map(address_json).collect())
}

fn address_json(address: &Address) -> serde_json::Value {
    let mut object = json!({ "email": address.email() });
    if let Some(name) = address.name() {
        object["name"] = json!(name);
    }
    object
}

fn truncate_body(body: &str) -> String {
    const MAX: usize = 200;
    let trimmed = body.trim();
    if trimmed.len() <= MAX {
        return trimmed.to_owned();
    }
    format!("{}...", &trimmed[..MAX])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Attachment, Email};

    fn sample_email() -> Email {
        Email::new()
            .from("Shop <shop@example.test>")
            .expect("from")
            .to("Buyer <buyer@example.test>")
            .expect("to")
            .cc("cc@example.test")
            .expect("cc")
            .bcc("bcc@example.test")
            .expect("bcc")
            .reply_to("reply@example.test")
            .expect("reply")
            .subject("Order")
            .text("plain thanks")
            .html("<p>thanks</p>")
    }

    #[test]
    fn config_authorization_is_bearer() {
        let config = EspApiConfig::new("https://api.example/mail/send", "sg-key");
        assert_eq!(config.endpoint_url(), "https://api.example/mail/send");
        assert_eq!(config.api_key(), "sg-key");
        assert_eq!(config.auth_scheme_value(), EspAuthScheme::Bearer);
        assert_eq!(config.authorization_header(), "Bearer sg-key");
    }

    #[test]
    fn payload_is_sendgrid_shaped() {
        let bytes = build_esp_payload(&sample_email()).expect("payload");
        let value: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        assert_eq!(value["from"]["email"], "shop@example.test");
        assert_eq!(value["from"]["name"], "Shop");
        assert_eq!(value["subject"], "Order");
        assert_eq!(value["reply_to"]["email"], "reply@example.test");
        assert_eq!(
            value["personalizations"][0]["to"][0]["email"],
            "buyer@example.test"
        );
        assert_eq!(value["personalizations"][0]["to"][0]["name"], "Buyer");
        assert_eq!(
            value["personalizations"][0]["cc"][0]["email"],
            "cc@example.test"
        );
        assert_eq!(
            value["personalizations"][0]["bcc"][0]["email"],
            "bcc@example.test"
        );
        assert_eq!(value["content"][0]["type"], "text/plain");
        assert_eq!(value["content"][1]["type"], "text/html");
    }

    #[test]
    fn payload_rejects_empty_body() {
        let email = Email::new()
            .from("a@b.test")
            .expect("from")
            .to("c@d.test")
            .expect("to")
            .subject("x");
        let err = build_esp_payload(&email).expect_err("body");
        assert!(matches!(err, MailerError::Transport { .. }));
    }

    #[test]
    fn transport_posts_and_accepts_2xx() {
        let poster = MockEspHttpPoster::accepted();
        let transport = EspHttpTransport::new(
            EspApiConfig::new("https://api.example/v3/mail/send", "key-1"),
            poster.clone(),
        );
        transport.send(&sample_email()).expect("send");
        let request = poster.last_request().expect("captured");
        assert_eq!(request.url, "https://api.example/v3/mail/send");
        assert!(
            request
                .headers
                .iter()
                .any(|(name, value)| name == "Authorization" && value == "Bearer key-1")
        );
        assert!(
            request
                .headers
                .iter()
                .any(|(name, value)| name == "Content-Type" && value == "application/json")
        );
        assert_ne!(request.body.len(), 0);
    }

    #[test]
    fn transport_rejects_non_2xx() {
        let poster = MockEspHttpPoster::new(EspHttpResponse::new(401, "unauthorized"));
        let transport =
            EspHttpTransport::new(EspApiConfig::new("https://api.example/send", "bad"), poster);
        let err = transport.send(&sample_email()).expect_err("fail");
        assert!(matches!(err, MailerError::Transport { message } if message.contains("401")));
    }

    #[test]
    fn transport_requires_sender() {
        let poster = MockEspHttpPoster::accepted();
        let transport =
            EspHttpTransport::new(EspApiConfig::new("https://api.example/send", "k"), poster);
        let email = Email::new().to("a@b.test").expect("to").text("x");
        let err = transport.send(&email).expect_err("sender");
        assert_eq!(err, MailerError::MissingSender);
    }

    #[test]
    fn response_success_range() {
        assert!(EspHttpResponse::new(200, "").is_success());
        assert!(EspHttpResponse::new(299, "").is_success());
        assert!(!EspHttpResponse::new(199, "").is_success());
        assert!(!EspHttpResponse::new(300, "").is_success());
    }

    #[test]
    fn attachments_do_not_break_payload() {
        let email = sample_email().attach(Attachment::from_bytes(
            "note.txt",
            "text/plain",
            b"hi".as_slice(),
        ));
        let bytes = build_esp_payload(&email).expect("payload");
        let value: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        assert!(value.get("attachments").is_none());
    }
}
