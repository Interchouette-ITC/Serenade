# Mailer

Email message, Mime lite types, and sync transports live in **`serenade-mailer`** ([#153](https://github.com/Interchouette-ITC/Serenade/issues/153), [#152](https://github.com/Interchouette-ITC/Serenade/issues/152)).

## Types

| Type | Role |
| --- | --- |
| `Address` | Mailbox string with optional display name |
| `Body` | Plain text and/or HTML |
| `Attachment` | Filename, MIME type, raw bytes |
| `Email` | Builder for headers, subject, body, attachments |
| `MailerError` | Build / send failures |

## Transports

| Transport | Role |
| --- | --- |
| `NullTransport` | Discards messages (default DI mailer) |
| `FileTransport` | Writes a readable dump under a directory |
| `SmtpTransport` | SMTP via lettre (Cargo feature `smtp`, on by default) |

All implement [`Transport`](https://docs.rs/serenade-mailer) with sync `send`.

## DI

`FrameworkExtension` adds [`RegisterDefaultMailerPass`], which registers service id `mailer` (`DEFAULT_MAILER_SERVICE`) tagged `mailer.transport` with a `NullTransport` when missing. Resolve `MailerService` and call `send`.

Apps replace the default by registering their own `mailer` service (file or SMTP) before compile.

## Example

```rust
use serenade_mailer::{Attachment, Email, FileTransport, NullTransport, Transport};

let email = Email::new()
    .from("shop@example.test")?
    .to("buyer@example.test")?
    .subject("Order confirmation")
    .text("Thanks for your order.")
    .html("<p>Thanks for your order.</p>")
    .attach(Attachment::from_bytes(
        "receipt.txt",
        "text/plain",
        b"order-1".as_slice(),
    ));

NullTransport::new().send(&email)?;
FileTransport::new("var/mail").send(&email)?;
```

SMTP (feature `smtp`):

```rust
use serenade_mailer::{SmtpTransport, Transport};

let smtp = SmtpTransport::relay("smtp.example.test")
    .credentials("user", "pass")
    .build()?;
smtp.send(&email)?;
```

## Related

- Parent epic: [#145](https://github.com/Interchouette-ITC/Serenade/issues/145)
