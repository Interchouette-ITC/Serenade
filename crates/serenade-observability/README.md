# serenade-observability

Structured logging on `tracing`: named channels, `LoggingConfig` / `init`, and
app-owned `var/log/{env}.log` file sinks.

Apps call `init` from `main`. See `docs-dev/OBSERVABILITY.md`.
