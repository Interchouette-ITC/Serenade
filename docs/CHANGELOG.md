# Changelog

All notable changes to Serenade are documented here. All `serenade-*` crates and the
`serenade-framework` facade share one version.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html). Raising the minimum
supported Rust version is treated as a minor change.

## [Unreleased]

### Fixed

- `serenade-bundle` no longer enables the HTTP-client and mailer default features.
  The facade's `bundle` feature now leaves reqwest and SMTP opt-in through
  `http-client-reqwest` and `mailer-smtp`.

## [1.0.0]

First release on crates.io.

### Added

- `serenade-framework`: public facade re-exporting a curated surface of the `serenade-*`
  crates behind Cargo features; the default build contains only kernel, config, DI, and
  events.
- Foundations: `serenade-kernel` (boot, environments, bundle lifecycle), `serenade-di`,
  `serenade-config` (TOML-first layered packages, dotenv), `serenade-event`,
  `serenade-bundle`, `serenade-contracts`.
- HTTP: framework-agnostic `serenade-http`, Actix and Axum adapters, `serenade-http-client`
  with an optional reqwest backend, sessions, security (firewall, voters, CSRF, password
  hashing, optional OAuth and LDAP), validation, rate limiting, and locks.
- Messaging and background work: `serenade-messenger`, `serenade-scheduler`,
  `serenade-mailer` (SMTP and ESP transports), `serenade-notifier`, `serenade-workflow`.
- Views: engine-neutral `serenade-view` with Leptos SSR and hydration support,
  `serenade-form` with CSRF by default, `serenade-minify`, `serenade-profiler`,
  `serenade-admin`.
- Supporting components: cache, serializer, translation, OpenAPI, search, string,
  filesystem, finder, process, expression, observability, Wasm hosts, and testing helpers.
- `serenade-cli`: the `serenade` executable (`new`, recipes, TUI) and SPA, HTML, Universal,
  and SSG front-mode tooling.
- Prebuilt `serenade-cli` binaries on the GitHub Release for Linux (x86_64, ARM64), macOS
  (Apple silicon, Intel), and Windows (x86_64), with SHA-256 checksums, build provenance
  attestations, and an optional verified Trunk bundle.
- Minimum supported Rust version 1.100.

[Unreleased]: https://github.com/Interchouette-ITC/Serenade/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/Interchouette-ITC/Serenade/releases/tag/v1.0.0
