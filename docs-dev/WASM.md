# Wasm host capability facade

Serenade provides **engine plumbing** for two Wasm host lanes. Product apps own WIT worlds, guest ABIs, fixtures, and proposal gates.

## Two lanes (do not merge)

| Lane | Crate | Engine | Typical use |
| --- | --- | --- | --- |
| Component Model plugins | `serenade-component-host` | **wasmtime** | Versioned WIT guests (product supplies `bindgen!` world) |
| Polyglot sandboxes | `serenade-sandbox` | **Wasmer** WASIX | Untrusted scripts / packages (stdin/stdout jobs) |

FrameworkBundle does **not** register Wasm hosts. Apps depend on the crates they need and wire runners themselves (same opt-in pattern as persistence adapters).

## Ownership

| Concern | Owner |
| --- | --- |
| Engine/Store/Linker helpers; load component from path | Serenade |
| Package cache root, `run_package` / `run_module`, `GuestOutput`, JSON decode | Serenade |
| WIT package names, commerce types, fixtures, validate-then-apply | Product |
| Cache env | `SERENADE_WASMER_CACHE` (product may alias its own env) |

## Bundle vs Wasm

See [BUNDLES.md](BUNDLES.md). In-process bundles remain the trusted DI story. Wasm guests stay sandboxed; Serenade only supplies the host runners.

## Related

- Epic [#115](https://github.com/Interchouette-ITC/Serenade/issues/115)
- Illustrative product map: [RUSTASHOP.md](RUSTASHOP.md)
