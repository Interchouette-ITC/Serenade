use std::sync::Arc;

use serenade_di::{CompilePass, ContainerBuilder, ServiceDefinition};
use tempfile::tempdir;

use super::*;

struct NamedAddress(Address);

impl TryInto<Address> for NamedAddress {
    type Error = MailerError;

    fn try_into(self) -> Result<Address, Self::Error> {
        Ok(self.0)
    }
}

fn named_address(email: &str, name: Option<&str>) -> NamedAddress {
    NamedAddress(
        Address::with_name(email, name).unwrap_or_else(|error| panic!("named address: {error:?}")),
    )
}

#[test]
fn address_rejects_empty_and_bare_at() {
    assert!(Address::new("").is_err());
    assert!(Address::new("@").is_err());
    assert!(Address::new("no-at").is_err());
    assert!(Address::new("@x").is_err());
    assert!(Address::new("x@").is_err());
}

#[test]
fn address_with_display_name() {
    let addr = Address::with_name("a@b.test", Some("Ada")).expect("valid");
    assert_eq!(addr.email(), "a@b.test");
    assert_eq!(addr.name(), Some("Ada"));
}

#[test]
fn email_builder_sets_headers_body_and_attachment() {
    let email = sample_email();

    assert_eq!(email.from_addresses()[0].email(), "from@example.test");
    assert_eq!(email.to_addresses()[0].email(), "to@example.test");
    assert_eq!(email.cc_addresses()[0].email(), "cc@example.test");
    assert_eq!(email.bcc_addresses()[0].email(), "bcc@example.test");
    assert_eq!(email.reply_to_addresses()[0].email(), "reply@example.test");
    assert_eq!(email.subject_line(), "Hello");
    assert_eq!(email.message_body().text_part(), Some("plain"));
    assert_eq!(email.message_body().html_part(), Some("<p>html</p>"));
    assert_eq!(email.attachments().len(), 1);
    assert_eq!(email.attachments()[0].filename(), "note.txt");
    assert_eq!(email.attachments()[0].body(), b"hi");
}

#[test]
fn body_helpers() {
    assert!(Body::empty().is_empty());
    assert_eq!(Body::text("a").text_part(), Some("a"));
    assert_eq!(Body::html("<b>").html_part(), Some("<b>"));
    let both = Body::both("t", "h");
    assert_eq!(both.text_part(), Some("t"));
    assert_eq!(both.html_part(), Some("h"));
}

#[test]
fn version_is_nonempty() {
    assert_ne!(version(), "");
}

#[test]
fn null_transport_accepts_valid_email() {
    NullTransport::new()
        .send(&sample_email())
        .expect("null send");
}

#[test]
fn null_transport_requires_sender_and_recipient() {
    let missing_from = Email::new().to("a@b.test").expect("to");
    assert_eq!(
        NullTransport::new().send(&missing_from),
        Err(MailerError::MissingSender)
    );
    let missing_to = Email::new().from("a@b.test").expect("from");
    assert_eq!(
        NullTransport::new().send(&missing_to),
        Err(MailerError::MissingRecipient)
    );
}

#[test]
fn file_transport_writes_message_dump() {
    let dir = tempdir().expect("tempdir");
    let transport = FileTransport::new(dir.path());
    transport.send(&sample_email()).expect("file send");
    let entries: Vec<_> = std::fs::read_dir(dir.path()).expect("read_dir").collect();
    assert_eq!(entries.len(), 1);
    let path = entries[0].as_ref().expect("entry").path();
    let contents = std::fs::read_to_string(path).expect("read");
    assert!(contents.contains("From: from@example.test"));
    assert!(contents.contains("Subject: Hello"));
    assert!(contents.contains("plain"));
}

#[test]
fn compile_pass_registers_null_mailer() {
    let pass = RegisterDefaultMailerPass;
    assert_eq!(pass.name(), "register_default_mailer");
    let mut builder = ContainerBuilder::new();
    builder.add_compile_pass(RegisterDefaultMailerPass);
    let container = builder.compile().expect("compile");
    let mailer = container
        .get_as::<MailerService>(DEFAULT_MAILER_SERVICE)
        .expect("mailer");
    mailer.send(&sample_email()).expect("send");
}

#[test]
fn compile_pass_skips_when_default_already_registered() {
    let mut builder = ContainerBuilder::new();
    builder
        .register(
            ServiceDefinition::new(DEFAULT_MAILER_SERVICE).with_tag(MAILER_TRANSPORT_TAG),
            |_| {
                Ok(Box::new(MailerService(
                    Arc::new(NullTransport::new()) as Arc<dyn Transport>
                )))
            },
        )
        .expect("register");
    builder.add_compile_pass(RegisterDefaultMailerPass);
    let container = builder.compile().expect("compile");
    assert!(
        container
            .get_as::<MailerService>(DEFAULT_MAILER_SERVICE)
            .is_ok()
    );
}

#[test]
fn address_parse_display_and_list() {
    let named = Address::parse(r"Ada Lovelace <ada@example.test>").expect("parse");
    assert_eq!(named.email(), "ada@example.test");
    assert_eq!(named.name(), Some("Ada Lovelace"));
    assert_eq!(named.to_string(), "Ada Lovelace <ada@example.test>");

    let list = Address::parse_list("a@b.test, Bob <c@d.test>").expect("list");
    assert_eq!(list.len(), 2);
    assert_eq!(list[1].name(), Some("Bob"));

    assert!(Address::parse("<>").is_err());
}

#[test]
fn email_embed_builds_related_mixed_tree() {
    let email = Email::new()
        .from("from@example.test")
        .expect("from")
        .to("to@example.test")
        .expect("to")
        .subject("Embed")
        .text("plain")
        .html("<img src=\"cid:logo@serenade\" />")
        .embed(Attachment::inline_from_bytes(
            "logo.png",
            "image/png",
            "logo@serenade",
            vec![1, 2, 3],
        ))
        .attach(Attachment::from_bytes(
            "note.txt",
            "text/plain",
            b"hi".as_slice(),
        ));

    assert_eq!(email.mime_tree().multipart_subtype(), Some("mixed"));
    assert!(email.attachments()[0].is_inline());
    assert_eq!(email.attachments()[0].content_id(), Some("logo@serenade"));
    assert!(!email.attachments()[1].is_inline());
}

#[test]
fn render_message_formats_named_multi_recipient_and_html_only() {
    use crate::render::render_message;

    let email = Email::new()
        .from(named_address("from@example.test", Some("From User")))
        .expect("from")
        .to(named_address("one@example.test", Some("One")))
        .expect("to")
        .to("two@example.test")
        .expect("to2")
        .subject("Hi")
        .html("<p>only html</p>");

    let dump = render_message(&email);
    assert!(dump.contains("From: From User <from@example.test>"));
    assert!(dump.contains("To: One <one@example.test>, two@example.test"));
    assert!(dump.contains("<p>only html</p>"));
    assert!(!dump.contains("-- html --"));
}

#[test]
fn render_message_shows_inline_and_mime_hint() {
    use crate::render::render_message;

    let email = Email::new()
        .from("from@example.test")
        .expect("from")
        .to("to@example.test")
        .expect("to")
        .subject("Hi")
        .html("<img src=\"cid:x\" />")
        .embed(Attachment::inline_from_bytes(
            "x.png",
            "image/png",
            "x",
            vec![9],
        ));
    let dump = render_message(&email);
    assert!(dump.contains("X-Serenade-Mime: multipart/related"));
    assert!(dump.contains("-- inline: x.png cid:x"));
}

#[test]
fn render_message_text_only_skips_optional_headers() {
    use crate::render::render_message;

    let email = Email::new()
        .from("from@example.test")
        .expect("from")
        .to("to@example.test")
        .expect("to")
        .subject("Plain")
        .text("body");

    let dump = render_message(&email);
    assert!(!dump.contains("Cc:"));
    assert!(!dump.contains("Bcc:"));
    assert!(dump.contains("body"));
}

#[cfg(feature = "smtp")]
#[test]
fn smtp_builder_dangerous_localhost() {
    let transport = SmtpTransport::unencrypted_localhost()
        .port(2525)
        .build()
        .expect("build");
    // No listener expected; connection failure is still a Transport error.
    let err = transport.send(&sample_email()).expect_err("no smtp");
    assert!(matches!(err, MailerError::Transport { .. }));
}

#[cfg(feature = "smtp")]
#[test]
fn smtp_send_success_against_loopback() {
    let port = spawn_loopback_smtp();
    let transport = SmtpTransport::unencrypted("127.0.0.1")
        .port(port)
        .build()
        .expect("transport");
    let email = Email::new()
        .from("from@example.test")
        .expect("from")
        .to("to@example.test")
        .expect("to")
        .subject("Hi")
        .text("body");
    transport.send(&email).expect("smtp send");
}

#[cfg(feature = "smtp")]
#[test]
fn smtp_send_validates_before_connect() {
    let transport = SmtpTransport::unencrypted_localhost()
        .port(2525)
        .build()
        .expect("build");
    let email = Email::new().to("to@example.test").expect("to");
    assert_eq!(transport.send(&email), Err(MailerError::MissingSender));
}

fn sample_email() -> Email {
    Email::new()
        .from("from@example.test")
        .expect("from")
        .to("to@example.test")
        .expect("to")
        .cc("cc@example.test")
        .expect("cc")
        .bcc("bcc@example.test")
        .expect("bcc")
        .reply_to("reply@example.test")
        .expect("reply")
        .subject("Hello")
        .text("plain")
        .html("<p>html</p>")
        .attach(Attachment::from_bytes(
            "note.txt",
            "text/plain",
            b"hi".as_slice(),
        ))
}

#[cfg(feature = "smtp")]
fn spawn_loopback_smtp() -> u16 {
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind smtp");
    let port = listener.local_addr().expect("addr").port();
    let (ready_tx, ready_rx) = mpsc::channel();
    thread::spawn(move || {
        ready_tx.send(()).expect("ready");
        for stream in listener.incoming() {
            let Ok(stream) = stream else {
                continue;
            };
            let _ = smtp_session(stream);
        }
    });
    ready_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("smtp thread");
    port
}

#[cfg(feature = "smtp")]
fn smtp_reply(stream: &mut std::net::TcpStream, code: u16, text: &str) -> std::io::Result<()> {
    use std::io::Write;
    write!(stream, "{code} {text}\r\n")?;
    stream.flush()
}

#[cfg(feature = "smtp")]
fn smtp_session(mut stream: std::net::TcpStream) -> std::io::Result<()> {
    use std::io::Read;

    let mut pending = String::new();
    let mut buf = [0u8; 1024];
    let mut in_data = false;

    smtp_reply(&mut stream, 220, "localhost ESMTP ready")?;
    loop {
        let read = stream.read(&mut buf)?;
        if read == 0 {
            break;
        }
        pending.push_str(&String::from_utf8_lossy(&buf[..read]));
        if in_data {
            if pending.contains("\r\n.\r\n") {
                in_data = false;
                pending.clear();
                smtp_reply(&mut stream, 250, "OK")?;
            }
            continue;
        }
        while let Some(pos) = pending.find("\r\n") {
            let line = pending[..pos].trim().to_owned();
            pending = pending[pos + 2..].to_owned();
            if line.is_empty() {
                continue;
            }
            let upper = line.to_uppercase();
            if upper.starts_with("EHLO") || upper.starts_with("HELO") {
                smtp_reply(&mut stream, 250, "localhost")?;
                smtp_reply(&mut stream, 250, "PIPELINING")?;
            } else if upper.starts_with("STARTTLS") {
                smtp_reply(&mut stream, 502, "Command not implemented")?;
            } else if upper == "DATA" {
                in_data = true;
                smtp_reply(&mut stream, 354, "End data with <CR><LF>.<CR><LF>")?;
            } else if upper.starts_with("QUIT") {
                smtp_reply(&mut stream, 221, "Bye")?;
                return Ok(());
            } else {
                smtp_reply(&mut stream, 250, "OK")?;
            }
        }
    }
    Ok(())
}
