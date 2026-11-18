# serenade-mailer

Email messages, MIME multipart trees, and sync transports (Symfony Mailer shaped).

Types: `Email`, `Address`, `Body`, `Attachment`, `MimeTree`. Transports:
`NullTransport`, `FileTransport`, `SmtpTransport` (default `smtp` feature), and
`EspHttpTransport` (`esp` feature). `RegisterDefaultMailerPass` wires the default
mailer service into the DI container.

Part of [Serenade](https://github.com/Interchouette-ITC/Serenade); most applications
depend on the [`serenade-framework`](https://crates.io/crates/serenade-framework) facade.
API documentation: <https://docs.rs/serenade-mailer>.

Licensed under Apache-2.0.
