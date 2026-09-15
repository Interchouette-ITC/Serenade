# serenade-observability

Structured logging on `tracing`: named channels, `LoggingConfig` / `init`, and
app-owned `var/log/{env}.log` file sinks.

Optional feature `otel` bridges spans to OpenTelemetry (in-process or OTLP/HTTP).

Apps call `init` (or `init_with_otel`) from `main`. See `docs-dev/OBSERVABILITY.md`.
