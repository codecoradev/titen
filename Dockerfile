# ── Titen — build-from-source Dockerfile (standalone / dev) ──────────────
# CI release builds use Dockerfile.release instead.
# Builds all three Rust binaries from source, then ships a minimal runtime.

# ── Stage 1: Builder ─────────────────────────────────────────────────────
FROM rust:1.88-slim-bookworm AS builder

# sqlx/sqlite bundling needs a C compiler; reqwest uses rustls (no openssl).
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential pkg-config ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Cache deps: copy manifests first, build deps, then copy source.
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# Build release binaries (3 crates produce titen-api, titen, titen-mcp).
RUN cargo build --release -p titen-api -p titen-cli -p titen-mcp && \
    strip target/release/titen-api target/release/titen target/release/titen-mcp

# ── Stage 2: Runtime ─────────────────────────────────────────────────────
FROM debian:trixie-slim

ARG VERSION=dev
LABEL org.opencontainers.image.title="titen"
LABEL org.opencontainers.image.version=${VERSION}

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3t64 libstdc++6 curl && \
    rm -rf /var/lib/apt/lists/*

# Non-root user (uid/gid 1000)
RUN groupadd --system --gid 1000 titen && \
    useradd --system --uid 1000 --gid titen --home-dir /data titen

COPY --from=builder /app/target/release/titen-api /usr/local/bin/titen-api
COPY --from=builder /app/target/release/titen       /usr/local/bin/titen
COPY --from=builder /app/target/release/titen-mcp   /usr/local/bin/titen-mcp

RUN mkdir -p /data && chown -R titen:titen /data

ENV TITEN_DB_PATH=/data/titen.db
ENV TITEN_HOST=0.0.0.0
ENV TITEN_PORT=7845

USER titen
WORKDIR /data
EXPOSE 7845

ENTRYPOINT ["titen-api"]
