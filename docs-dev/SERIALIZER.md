# Serializer

DTO ↔ wire formats live in **`serenade-serializer`**.

## Formats

| Format | Cargo feature | Typical consumer |
| --- | --- | --- |
| **JSON** (`FORMAT_JSON`) | always on | HTTP, OpenAPI, MCP tool wire |
| **TOON** v4.1 (`FORMAT_TOON`) | `toon` | LLM / agent **context export** (prompt packs) |

TOON is not a config format and does not replace JSON on the wire. Same intermediate `serde_json::Value` hub; apps enable `serenade-serializer/toon` and call `encode_toon_string` (or `serialize(..., "toon")`) when stuffing large structured dumps into model messages.

Spec: [TOON v4.1](https://github.com/toon-format/spec). Rust codec: `reddb-io-toon`.

## Pieces

| Type | Role |
| --- | --- |
| `Encoder` / `Decoder` | `Value` ↔ bytes for a named format |
| `JsonEncoder` / `JsonDecoder` | Built-in JSON |
| `ToonEncoder` / `ToonDecoder` | Built-in TOON when feature `toon` |
| `encode_toon_string` | UTF-8 TOON string for `system` / `user` prompts |
| `Normalizer` / `Denormalizer` / `NormalizerRegistry` | Object ↔ `Value` |
| `Serializer` | Normalize then encode; decode then denormalize |
| Serde bridge | Typed JSON helpers (`serialize_value` / `deserialize_value`) |

## Enable TOON

```toml
serenade-serializer = { version = "0.1", features = ["toon"] }
```

```rust
use serde_json::json;
use serenade_serializer::encode_toon_string;

let context = encode_toon_string(&json!({ "sku": "HOODIE", "qty": 2 }))?;
// inject `context` into an LLM message; keep MCP/HTTP payloads as JSON
```

## Non-goals

- SOAP / XML “web services”
- TOON as application config (use TOML / YAML)
- Auto-enabling TOON in FrameworkBundle (apps opt in)
