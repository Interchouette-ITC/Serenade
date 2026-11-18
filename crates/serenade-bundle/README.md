# serenade-bundle

Bundle interface and composition helpers (`Extension`, `FrameworkBundle`,
container build).

Applications register bundles the way Symfony apps load `FrameworkBundle`.

The HTTP-client and mailer contracts are included without their default features.
Bundle composition does not enable reqwest or SMTP. `FrameworkExtension` keeps
the null mailer, and only registers a default HTTP client when reqwest is enabled.

With `serenade-framework`, opt in to `http-client-reqwest` or `mailer-smtp`
alongside `bundle` to enable those integrations. Direct consumers can enable
`serenade-http-client/reqwest` or `serenade-mailer/smtp` in their dependencies.
