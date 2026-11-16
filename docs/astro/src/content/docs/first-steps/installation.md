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
`wasm32-unknown-unknown` target. Serenade installs and caches its managed Trunk
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

The first SPA, SSG, or Universal frontend build downloads Serenade's managed Trunk:
a prebuilt binary from the rolling `trunk-itc-dev` release of the
[Interchouette Trunk fork](https://github.com/Interchouette-ITC/trunk) (an MIT/Apache-2.0
fork of [Trunk](https://github.com/trunk-rs/trunk)). Prebuilt binaries cover Linux x86_64,
macOS x86_64/ARM64, and Windows x86_64. The download is verified against its published
SHA-256 checksum, its GitHub build-provenance attestation, and `trunk --version` before
use, and cached under
`~/.serenade/tools`; set `SERENADE_HOME` to use another cache location. This is
separate from any system-wide Trunk installation. Serenade checks for a newer build at
most once a day and keeps using the cached one when offline; set
`SERENADE_TRUNK_REFRESH=1` to check now. To run Trunk directly with
Serenade's managed version, use
`cargo run -p serenade-cli --bin serenade-trunk -- <trunk arguments>`.
Fork builds report their source commit, e.g. `trunk 0.22.1+itc.0123456789ab`; Serenade
only installs builds carrying that revision and records it in
`~/.serenade/tools/trunk-itc-dev/.serenade-trunk-version` (also shown by
`serenade-trunk --version`).
The checksum only detects corruption; the attestation proves who built the binary.
Serenade fetches it from GitHub's attestations API and verifies it offline with
Sigstore's public-good trust root: it must be SLSA build provenance signed by
`Interchouette-ITC/trunk/.github/workflows/release.yaml` on `refs/heads/main`, and its
source commit must match the `+itc.<revision>` the binary reports. The commit is
recorded in `~/.serenade/tools/trunk-itc-dev/.serenade-trunk-provenance`. If the
attestation is missing, invalid, or cannot be fetched, the new build is not installed
and the command fails. Anonymous API calls are limited to 60 per hour; set
`GITHUB_TOKEN` or `GH_TOKEN` to raise that limit. The same check can be run by hand
with `gh attestation verify <archive> --repo Interchouette-ITC/trunk --signer-workflow
Interchouette-ITC/trunk/.github/workflows/release.yaml`.
Concurrent builds share an OS installation lock, released automatically if the installer
exits or is killed. The persistent `.install-lock` file is reusable; it does not need
manual deletion after an interrupted installation.

## Next

- [Run the demo application](../first-application/) to see a bundle-based app boot.
- [Choose a web mode](../choose-your-mode/) to run the HTML, SPA, SSG, or Universal example.
- Read [the architecture](../../concepts/architecture/) before assembling an application.
