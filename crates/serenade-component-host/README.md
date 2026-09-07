# serenade-component-host

Wasmtime **Component Model** host helpers for Serenade apps.

Product crates own WIT worlds and `bindgen!` call sites. This crate supplies
Engine / Store / empty-Linker helpers and component load-from-path.

Enable with Cargo feature `wasmtime` (default).

See `docs-dev/WASM.md`.
