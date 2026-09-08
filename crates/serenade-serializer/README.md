# serenade-serializer

DTO serialization with JSON codecs, optional TOON v4.1, and a normalizer registry.

Normalize domain objects to arrays/maps for HTTP or messenger payloads without
tying the kernel to a single serde layout. Enable feature `toon` for
token-efficient LLM / agent context export (`FORMAT_TOON`, `encode_toon_string`).

See `docs-dev/SERIALIZER.md`.
