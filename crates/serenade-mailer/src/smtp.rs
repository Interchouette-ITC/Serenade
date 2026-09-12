//! SMTP transport (lettre).

use lettre::message::{Mailbox, Message, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{SmtpTransport as LettreSmtp, Transport as LettreTransport};

use crate::null::validate_for_send;
use crate::{Address, Email, MailerError, Transport};

/// SMTP relay transport (Symfony `SmtpTransport`).
#[derive(Clone, Debug)]
pub struct SmtpTransport {
    inner: LettreSmtp,
}

/// Builder for [`SmtpTransport`].
#[derive(Clone, Debug)]
pub struct SmtpTransportBuilder {
    relay: String,
    port: Option<u16>,
    credentials: Option<(String, String)>,
    unencrypted: bool,
}

impl SmtpTransport {
    /// Starts a builder for TLS relay host `relay` (lettre default port).
    #[must_use]
    pub fn relay(relay: impl Into<String>) -> SmtpTransportBuilder {
        SmtpTransportBuilder {
            relay: relay.into(),
            port: None,
            credentials: None,
            unencrypted: false,
        }
    }

    /// Starts a builder for cleartext SMTP to `host` (local / test relays).
    #[must_use]
    pub fn unencrypted_localhost() -> SmtpTransportBuilder {
        SmtpTransportBuilder {
            relay: "localhost".into(),
            port: Some(25),
            credentials: None,
            unencrypted: true,
        }
    }
}

impl SmtpTransportBuilder {
    /// Sets the SMTP port.
    #[must_use]
    pub const fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Sets PLAIN credentials.
    #[must_use]
    pub fn credentials(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.credentials = Some((username.into(), password.into()));
        self
    }

    /// Builds the transport.
    ///
    /// # Errors
    ///
    /// Returns [`MailerError::Transport`] when the SMTP client cannot be created.
    pub fn build(self) -> Result<SmtpTransport, MailerError> {
        let mut builder = if self.unencrypted {
            LettreSmtp::builder_dangerous(self.relay)
        } else {
            LettreSmtp::relay(&self.relay).map_err(|error| MailerError::Transport {
                message: error.to_string(),
            })?
        };
        if let Some(port) = self.port {
            builder = builder.port(port);
        }
        if let Some((username, password)) = self.credentials {
            builder = builder.credentials(Credentials::new(username, password));
        }
        Ok(SmtpTransport {
            inner: builder.build(),
        })
    }
}

impl Transport for SmtpTransport {
    fn send(&self, email: &Email) -> Result<(), MailerError> {
        validate_for_send(email)?;
        let message = to_lettre_message(email)?;
        self.inner
            .send(&message)
            .map_err(|error| MailerError::Transport {
                message: error.to_string(),
            })?;
        Ok(())
    }
}

fn to_lettre_message(email: &Email) -> Result<Message, MailerError> {
    let mut builder = Message::builder();
    for address in email.from_addresses() {
        builder = builder.from(to_mailbox(address)?);
    }
    for address in email.to_addresses() {
        builder = builder.to(to_mailbox(address)?);
    }
    for address in email.cc_addresses() {
        builder = builder.cc(to_mailbox(address)?);
    }
    for address in email.bcc_addresses() {
        builder = builder.bcc(to_mailbox(address)?);
    }
    for address in email.reply_to_addresses() {
        builder = builder.reply_to(to_mailbox(address)?);
    }
    builder = builder.subject(email.subject_line());

    let text = email.message_body().text_part();
    let html = email.message_body().html_part();
    let multipart = match (text, html) {
        (Some(text), Some(html)) => MultiPart::alternative()
            .singlepart(SinglePart::plain(text.to_owned()))
            .singlepart(SinglePart::html(html.to_owned())),
        (Some(text), None) => MultiPart::mixed().singlepart(SinglePart::plain(text.to_owned())),
        (None, Some(html)) => MultiPart::mixed().singlepart(SinglePart::html(html.to_owned())),
        (None, None) => MultiPart::mixed().singlepart(SinglePart::plain(String::new())),
    };

    let mut multipart = multipart;
    for attachment in email.attachments() {
        multipart = multipart.singlepart(
            SinglePart::builder()
                .header(
                    lettre::message::header::ContentType::parse(attachment.content_type())
                        .map_err(|error| MailerError::Transport {
                            message: error.to_string(),
                        })?,
                )
                .header(lettre::message::header::ContentDisposition::attachment(
                    attachment.filename(),
                ))
                .body(attachment.body().to_vec()),
        );
    }

    builder
        .multipart(multipart)
        .map_err(|error| MailerError::Transport {
            message: error.to_string(),
        })
}

fn to_mailbox(address: &Address) -> Result<Mailbox, MailerError> {
    let email: lettre::Address =
        address
            .email()
            .parse()
            .map_err(|error| MailerError::Transport {
                message: format!("invalid mailbox {}: {error}", address.email()),
            })?;
    Ok(Mailbox::new(address.name().map(str::to_owned), email))
}
