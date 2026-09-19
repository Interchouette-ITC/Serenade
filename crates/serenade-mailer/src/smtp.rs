//! SMTP transport (lettre).

use lettre::message::{Attachment as LettreAttachment, Mailbox, Message, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{SmtpTransport as LettreSmtp, Transport as LettreTransport};

use crate::mime::{MimePart, MimeTree};
use crate::null::validate_for_send;
use crate::{Address, ContentDisposition, Email, MailerError, Transport};

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
    pub fn unencrypted(host: impl Into<String>) -> SmtpTransportBuilder {
        SmtpTransportBuilder {
            relay: host.into(),
            port: None,
            credentials: None,
            unencrypted: true,
        }
    }

    /// Cleartext SMTP to `localhost` (default port 25).
    #[must_use]
    pub fn unencrypted_localhost() -> SmtpTransportBuilder {
        Self::unencrypted("localhost").port(25)
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

    let tree = email.mime_tree();
    match tree {
        MimeTree::Single(part) => {
            builder
                .singlepart(single_part_from_mime(&part)?)
                .map_err(|error| MailerError::Transport {
                    message: error.to_string(),
                })
        }
        other => builder
            .multipart(multipart_from_tree(&other)?)
            .map_err(|error| MailerError::Transport {
                message: error.to_string(),
            }),
    }
}

fn multipart_from_tree(tree: &MimeTree) -> Result<MultiPart, MailerError> {
    match tree {
        MimeTree::Single(part) => Ok(MultiPart::mixed().singlepart(single_part_from_mime(part)?)),
        MimeTree::Alternative { text, html } => build_alternative(text.as_ref(), html),
        MimeTree::Related { root, related } => build_related(root, related),
        MimeTree::Mixed { body, attachments } => build_mixed(body, attachments),
    }
}

fn build_alternative(text: Option<&MimePart>, html: &MimeTree) -> Result<MultiPart, MailerError> {
    match html {
        MimeTree::Single(part) => {
            if let Some(text_part) = text {
                Ok(MultiPart::alternative()
                    .singlepart(single_part_from_mime(text_part)?)
                    .singlepart(single_part_from_mime(part)?))
            } else {
                Ok(MultiPart::alternative().singlepart(single_part_from_mime(part)?))
            }
        }
        nested => {
            let related = multipart_from_tree(nested)?;
            if let Some(text_part) = text {
                Ok(MultiPart::alternative()
                    .singlepart(single_part_from_mime(text_part)?)
                    .multipart(related))
            } else {
                Ok(related)
            }
        }
    }
}

fn build_related(root: &MimeTree, related: &[MimePart]) -> Result<MultiPart, MailerError> {
    let mut multipart = match root {
        MimeTree::Single(part) => MultiPart::related().singlepart(single_part_from_mime(part)?),
        nested => MultiPart::related().multipart(multipart_from_tree(nested)?),
    };
    for part in related {
        multipart = multipart.singlepart(file_part(part)?);
    }
    Ok(multipart)
}

fn build_mixed(body: &MimeTree, attachments: &[MimePart]) -> Result<MultiPart, MailerError> {
    let mut multipart = match body {
        MimeTree::Single(part) => MultiPart::mixed().singlepart(single_part_from_mime(part)?),
        nested => MultiPart::mixed().multipart(multipart_from_tree(nested)?),
    };
    for part in attachments {
        multipart = multipart.singlepart(file_part(part)?);
    }
    Ok(multipart)
}

fn single_part_from_mime(part: &MimePart) -> Result<SinglePart, MailerError> {
    if part.content_type.starts_with("text/html") {
        let body = String::from_utf8_lossy(&part.body).into_owned();
        return Ok(SinglePart::html(body));
    }
    if part.content_type.starts_with("text/plain") {
        let body = String::from_utf8_lossy(&part.body).into_owned();
        return Ok(SinglePart::plain(body));
    }
    file_part(part)
}

fn file_part(part: &MimePart) -> Result<SinglePart, MailerError> {
    let content_type =
        lettre::message::header::ContentType::parse(&part.content_type).map_err(|error| {
            MailerError::Transport {
                message: error.to_string(),
            }
        })?;
    let filename = part
        .filename
        .clone()
        .unwrap_or_else(|| "part.bin".to_owned());
    match part.disposition {
        ContentDisposition::Inline => {
            let content_id = part.content_id.clone().unwrap_or_else(|| filename.clone());
            Ok(LettreAttachment::new_inline_with_name(content_id, filename)
                .body(part.body.clone(), content_type))
        }
        ContentDisposition::Attachment => {
            Ok(LettreAttachment::new(filename).body(part.body.clone(), content_type))
        }
    }
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

#[cfg(all(test, feature = "smtp"))]
mod smtp_tests {
    use super::{Address, Email, MailerError, SmtpTransport, to_lettre_message, to_mailbox};
    use crate::Attachment;

    #[test]
    fn relay_and_credentials_builders() {
        SmtpTransport::relay("smtp.example.test")
            .build()
            .expect("tls relay build");
        SmtpTransport::unencrypted("127.0.0.1")
            .build()
            .expect("cleartext build without explicit port");
        SmtpTransport::unencrypted("127.0.0.1")
            .credentials("user", "secret")
            .port(2525)
            .build()
            .expect("credentials build");
    }

    #[test]
    fn to_lettre_message_body_shapes_and_attachment() {
        let text_only = Email::new()
            .from("from@example.test")
            .expect("from")
            .to("to@example.test")
            .expect("to")
            .subject("Text")
            .text("plain");
        to_lettre_message(&text_only).expect("text message");

        let html_only = Email::new()
            .from("from@example.test")
            .expect("from")
            .to("to@example.test")
            .expect("to")
            .subject("Html")
            .html("<p>x</p>");
        to_lettre_message(&html_only).expect("html message");

        let empty = Email::new()
            .from("from@example.test")
            .expect("from")
            .to("to@example.test")
            .expect("to")
            .subject("Empty");
        to_lettre_message(&empty).expect("empty message");

        let with_embed = Email::new()
            .from("from@example.test")
            .expect("from")
            .to("to@example.test")
            .expect("to")
            .subject("Embed")
            .text("plain")
            .html("<img src=\"cid:logo\" />")
            .embed(Attachment::inline_from_bytes(
                "logo.png",
                "image/png",
                "logo",
                vec![1, 2, 3],
            ))
            .attach(Attachment::from_bytes("a.txt", "text/plain", b"x"));
        let message = to_lettre_message(&with_embed).expect("multipart message");
        let formatted = message.formatted();
        let encoded = String::from_utf8_lossy(&formatted);
        assert!(encoded.contains("multipart/mixed"));
        assert!(encoded.contains("multipart/alternative"));
        assert!(encoded.contains("multipart/related"));
        assert!(encoded.contains("Content-ID: <logo>"));
        assert!(encoded.contains("filename=\"a.txt\""));

        let named = Email::new()
            .from("from@example.test")
            .expect("from")
            .to("to@example.test")
            .expect("to")
            .subject("Named");
        to_mailbox(&Address::with_name("ada@example.test", Some("Ada Lovelace")).expect("named"))
            .expect("named mailbox");
        let _ = named;
    }

    #[test]
    fn to_lettre_message_rejects_bad_mailbox_and_content_type() {
        let bad_mailbox = Email::new()
            .from("a@b@c")
            .expect("from")
            .to("to@example.test")
            .expect("to")
            .subject("Bad");
        assert!(matches!(
            to_lettre_message(&bad_mailbox),
            Err(MailerError::Transport { .. })
        ));

        let bad_type = Email::new()
            .from("from@example.test")
            .expect("from")
            .to("to@example.test")
            .expect("to")
            .subject("Bad type")
            .attach(Attachment::from_bytes("x.bin", "!!!", b"x"));
        assert!(matches!(
            to_lettre_message(&bad_type),
            Err(MailerError::Transport { .. })
        ));
    }

    #[test]
    fn to_lettre_message_requires_from_for_single_and_multipart() {
        let text_only = Email::new()
            .to("to@example.test")
            .expect("to")
            .subject("Text")
            .text("plain");
        assert!(matches!(
            to_lettre_message(&text_only),
            Err(MailerError::Transport { .. })
        ));

        let multipart = Email::new()
            .to("to@example.test")
            .expect("to")
            .subject("Multi")
            .text("plain")
            .html("<p>x</p>");
        assert!(matches!(
            to_lettre_message(&multipart),
            Err(MailerError::Transport { .. })
        ));
    }

    #[test]
    fn multipart_helpers_cover_edge_trees() {
        use super::{
            ContentDisposition, MimePart, MimeTree, build_alternative, build_related,
            multipart_from_tree, single_part_from_mime,
        };

        let plain = MimePart::text("hi");
        let html = MimePart::html("<p>x</p>");
        let binary = MimePart {
            content_type: "application/octet-stream".to_owned(),
            filename: Some("blob.bin".to_owned()),
            disposition: ContentDisposition::Attachment,
            content_id: None,
            body: vec![0, 1, 2],
        };

        multipart_from_tree(&MimeTree::Single(plain.clone())).expect("single via multipart");
        build_alternative(None, &MimeTree::Single(html.clone())).expect("html-only alternative");
        let related = MimeTree::Related {
            root: Box::new(MimeTree::Single(html.clone())),
            related: vec![MimePart::from_attachment(&Attachment::inline_from_bytes(
                "i.png",
                "image/png",
                "i",
                vec![9],
            ))],
        };
        build_alternative(None, &related).expect("related without text alternative");
        build_related(
            &MimeTree::Alternative {
                text: Some(plain),
                html: Box::new(MimeTree::Single(html)),
            },
            &[],
        )
        .expect("related around alternative root");
        single_part_from_mime(&binary).expect("binary single part");
    }
}
