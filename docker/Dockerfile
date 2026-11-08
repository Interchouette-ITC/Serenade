# Build from the repository root:
#   docker build -f docker/Dockerfile -t serenade/myfeed:latest .

ARG RUST_IMAGE=rust:1.99.1-bookworm
ARG RUNTIME_IMAGE=debian:bookworm-slim

FROM ${RUST_IMAGE} AS builder

WORKDIR /app

COPY . .

ENV CARGO_TERM_COLOR=never \
    CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --release --locked -p serenade-demo-html

FROM ${RUNTIME_IMAGE} AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --system --uid 10001 --create-home app \
    && mkdir -p /data /app/examples/demo-html \
    && chown app:app /data /app/examples/demo-html

COPY --from=builder /app/target/release/serenade-demo-html /usr/local/bin/myfeed
COPY --from=builder --chown=app:app /app/examples/demo-html/translations /app/examples/demo-html/translations

ENV MYFEED_BIND=0.0.0.0:8080 \
    MYFEED_DB=/data/myfeed.sqlite

USER 10001
EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/myfeed"]
