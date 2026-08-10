# Titen — multi-target Dockerfile
#
# Usage:
#   docker build --target web -t titen-web .       # SvelteKit SSR via Bun
#   docker build --target api -t titen-api .        # Rust Axum API
#
# By default, builds the API target (backward compatible).

# ── Shared: Build the SvelteKit frontend ──────────────────────────────
# adapter-node generates build/server/index.js + build/client/.
FROM oven/bun:1-alpine AS frontend
WORKDIR /app/web
COPY web/package.json web/bun.lock ./
RUN bun install --frozen-lockfile
COPY web/ ./
RUN bun run build

# ── Web target: SvelteKit SSR server via Bun ─────────────────────────
FROM oven/bun:1-alpine AS web

WORKDIR /app
COPY --from=frontend /app/web/build ./
# Runtime deps that Vite 8 does not bundle into SSR output.
# adapter-node generates chunks that import these at runtime from
# node_modules, so they must be present in the image.
COPY --from=frontend /app/web/node_modules ./node_modules

# Run as non-root user (bun image ships with `bun` user uid 1000)
USER bun

# adapter-node reads HOST, PORT, ORIGIN from environment.
ENV HOST=0.0.0.0
ENV PORT=3000
ENV ORIGIN=http://localhost:3000

EXPOSE 3000

ENTRYPOINT ["bun", "run", "index.js"]

# ── API target: Rust Axum API (backend-only, no static files) ──────────
FROM rust:1.88-alpine AS api-builder

RUN apk add --no-cache \
        musl-dev \
        cmake \
        make \
        perl \
        clang \
        llvm-dev \
        libgcc \
        curl

ENV CC=clang CXX=clang++

WORKDIR /app

# Cache dependencies — copy only Cargo files first and build a dummy crate.
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
RUN cargo build --release --bins

# ── API runtime ──────────────────────────────────────────────────────
FROM alpine:3.24 AS api

RUN apk add --no-cache ca-certificates libgcc libstdc++ \
    && addgroup -S -g 1000 titen \
    && adduser -S -D -H -u 1000 -G titen titen

# Static binary + CLI tools.
COPY --from=api-builder --chown=titen:titen /app/target/release/titen-api /usr/local/bin/titen-api
COPY --from=api-builder --chown=titen:titen /app/target/release/titen /usr/local/bin/titen
COPY --from=api-builder --chown=titen:titen /app/target/release/titen-mcp /usr/local/bin/titen-mcp

# SQLite database lives on a bind-mounted volume (see docker-compose.yml).
VOLUME /data

ENV TITEN_DB_PATH=/data/titen.db
ENV TITEN_HOST=0.0.0.0
ENV TITEN_PORT=7845
ENV RUST_LOG=titen_api=info,tower_http=info

USER titen

EXPOSE 7845

ENTRYPOINT ["/usr/local/bin/titen-api"]
