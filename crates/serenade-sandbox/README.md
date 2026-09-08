# serenade-sandbox

Wasmer **WASIX** host helpers for Serenade apps.

Product crates own guest ABIs, package URL pins, and fixtures. This crate
supplies package/module runners, cache root (`SERENADE_WASMER_CACHE`), and
`GuestOutput` / JSON decode.

Enable with Cargo feature `wasix` (default).

See `docs-dev/WASM.md`.
