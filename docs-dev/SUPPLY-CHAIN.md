# Supply-chain checks

Local and CI gates for Rust dependencies.

Requires once: `cargo install cargo-audit cargo-deny`

```bash
make audit   # cargo audit
make deny    # cargo deny check (deny.toml)
```

Add a documented ignore only when an advisory is accepted with a written reason.

Current documented ignore: `RUSTSEC-2026-0258` (h2 0.3 via actix-http 3.x; patched releases are on h2 0.4.x only).
