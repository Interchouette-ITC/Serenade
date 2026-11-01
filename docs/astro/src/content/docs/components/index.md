---
title: Components
description: Explore Serenade’s workspace crates by responsibility and follow each crate to its source.
---

Serenade is a collection of focused Rust crates, not one mandatory full-stack dependency. The
workspace contains framework foundations, adapters, application services, and supporting tools.

## Start with the foundation

- **Application composition:** `serenade-kernel`, `serenade-bundle`, `serenade-di`,
  `serenade-config`, and `serenade-event`.
- **HTTP:** `serenade-http`, with separate `serenade-http-actix` and `serenade-http-axum`
  adapters.
- **Application capabilities:** security, forms, sessions, cache, messenger, mailer, notifier,
  serializer, validator, and more.
- **Supporting tools:** CLI and console, observability, testing, filesystem, process, and workflow
  crates.

## Browse the sources

The [workspace crate catalog](./catalog/) is generated from the actual
Cargo workspace members and crate manifests. Each entry links to its source and, when present,
its README. Crate-level API reference remains in rustdoc.

All workspace crates are pre-1.0. The catalog labels readiness as unassessed because the project
does not maintain a per-crate readiness policy; version and workspace membership are not
production-readiness guarantees. Check each crate’s README, tests, and examples before adopting it.

The generated [Rust API reference](../reference/api-docs/) complements these human-oriented
guides; it does not replace them.
