---
title: Installation
description: Check the Rust toolchain and prepare a local Serenade workspace.
---

Serenade is currently developed as a Rust workspace rather than a published, versioned set of
crates. Start from the repository so the examples and local crate versions match.

## Prerequisites

- Rust **1.99.0 or newer**, as specified by the workspace manifest.
- Git.
- GNU Make for the documented workspace commands.

To build the browser examples, Serenade needs Git, Cargo, and the
`wasm32-unknown-unknown` target. Serenade installs and caches its pinned Trunk
version automatically on the first frontend build.

## Clone the workspace

```sh
git clone https://github.com/Interchouette-ITC/Serenade.git
cd Serenade
make test
```

The first Cargo build downloads the workspace dependencies. To build the browser examples later:

```sh
rustup target add wasm32-unknown-unknown
```

The first SPA, SSG, or Universal frontend build downloads and compiles Serenade's
pinned Trunk release (`v0.22.1-itc.2`). It is cached under
`~/.serenade/tools`; set `SERENADE_HOME` to use another cache location. This is
separate from any system-wide Trunk installation. To run Trunk directly with
Serenade's pinned version, use
`cargo run -p serenade-cli --bin serenade-trunk -- <trunk arguments>`.

## Next

- [Run the demo application](../first-application/) to see a bundle-based app boot.
- [Choose a web mode](../choose-your-mode/) to run the HTML, SPA, SSG, or Universal example.
- Read [the architecture](../../concepts/architecture/) before assembling an application.
