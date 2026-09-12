use std::sync::Arc;

use serenade_di::{CompilePass, ContainerBuilder, ServiceDefinition};
use tempfile::tempdir;

use super::*;

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
