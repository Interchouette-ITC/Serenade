# Mailer

Email message, Mime multipart types, and sync transports live in **`serenade-mailer`** ([#153](https://github.com/Interchouette-ITC/Serenade/issues/153), [#152](https://github.com/Interchouette-ITC/Serenade/issues/152), [#228](https://github.com/Interchouette-ITC/Serenade/issues/228), [#253](https://github.com/Interchouette-ITC/Serenade/issues/253)).

## Types

| Type | Role |
| --- | --- |
| `Address` | Mailbox (`email@host` or `Name <email@host>`), `parse` / `parse_list`, `Display` |
| `Body` | Plain text and/or HTML |
| `Attachment` | Downloadable or inline (CID) part: filename, MIME type, bytes, disposition |
| `ContentDisposition` | `Attachment` vs `Inline` |
| `Email` | Builder for headers, subject, body, `attach` / `embed` |
| `MimeTree` / `MimePart` | Multipart layout: `alternative` / `related` / `mixed` |
| `MailerError` | Build / send failures |

`Email::mime_tree()` returns the nested structure SMTP encode uses:

- text + HTML → `multipart/alternative`
- HTML + CID embeds → `multipart/related` (HTML alternative sibling when text is present)
- downloadable files → outer `multipart/mixed`

## Transports

| Transport | Role |
| --- | --- |
| `NullTransport` | Discards messages (default DI mailer) |
| `FileTransport` | Writes a readable dump under a directory |
| `SmtpTransport` | SMTP via lettre (Cargo feature `smtp`, on by default) |
| `EspHttpTransport` | HTTP ESP mail API (Cargo feature `esp`, off by default) |

All implement [`Transport`](https://docs.rs/serenade-mailer) with sync `send`.

## DI

`FrameworkExtension` adds [`RegisterDefaultMailerPass`], which registers service id `mailer` (`DEFAULT_MAILER_SERVICE`) tagged `mailer.transport` with a `NullTransport` when missing. Resolve `MailerService` and call `send`.

Apps replace the default by registering their own `mailer` service (file or SMTP) before compile.

## Example

```rust
use serenade_mailer::{Attachment, Email, FileTransport, NullTransport, Transport};

let email = Email::new()
    .from("Shop <shop@example.test>")?
    .to("buyer@example.test")?
    .subject("Order confirmation")
    .text("Thanks for your order.")
    .html("<p>Thanks <img src=\"cid:logo@shop\" /></p>")
    .embed(Attachment::inline_from_bytes(
        "logo.png",
        "image/png",
        "logo@shop",
        b"\x89PNG".as_slice(),
    ))
    .attach(Attachment::from_bytes(
        "receipt.txt",
        "text/plain",
        b"order-1".as_slice(),
    ));

assert_eq!(email.mime_tree().multipart_subtype(), Some("mixed"));

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

## ESP HTTP (feature `esp`)

Enable with `serenade-mailer` feature `esp`. Serenade shapes a **SendGrid-class** JSON mail
API and a sync [`EspHttpPoster`] trait; apps own the HTTP client (no mandatory vendor SDK).

| Piece | Role |
| --- | --- |
| `EspApiConfig` | Endpoint URL + API key (`Authorization: Bearer <api_key>`) |
| `EspHttpPoster` / `MockEspHttpPoster` | Sync POST trait + test double |
| `EspHttpTransport` | Builds JSON from `Email`, POSTs, requires 2xx |
| `build_esp_payload` | Shared JSON encoder (text/HTML; attachments omitted) |

```rust
use serenade_mailer::{
    EspApiConfig, EspHttpTransport, MockEspHttpPoster, Transport,
};

let poster = MockEspHttpPoster::accepted();
let transport = EspHttpTransport::new(
    EspApiConfig::new("https://api.example/v3/mail/send", "api-key"),
    poster,
);
transport.send(&email)?;
```

## Related

- Parent epic: [#145](https://github.com/Interchouette-ITC/Serenade/issues/145)
- Mime deepen: [#228](https://github.com/Interchouette-ITC/Serenade/issues/228)
- ESP HTTP: [#253](https://github.com/Interchouette-ITC/Serenade/issues/253)
