# Titen — multi-target Dockerfile
#
# Usage:
#   docker build --target web -t titen-web .       # SvelteKit SSR via Bun
#   docker build --target api -t titen-api .       # Rust Axum API
#
# By default, builds the API target (backward compatible).

# ── Shared: Build the SvelteKit frontend ──────────────────────────────
# Both the web and api targets depend on a built frontend.
# adapter-node generates build/server/index.js + build/client/.
FROM oven/bun:1 AS frontend
WORKDIR /app/web
COPY web/package.json web/bun.lock ./
RUN bun install --frozen-lockfile
COPY web/ ./
RUN bun run build

# ── Web target: SvelteKit SSR server via Bun ─────────────────────────
FROM oven/bun:1 AS web

WORKDIR /app
COPY --from=frontend /app/web/build .
# Runtime deps that Vite does not bundle into SSR output.
# adapter-node generates chunks that import these at runtime from node_modules.
COPY --from=frontend /app/web/node_modules ./node_modules

# adapter-node reads HOST, PORT, ORIGIN from environment.
ENV HOST=0.0.0.0
ENV PORT=3000
ENV ORIGIN=http://localhost:3000

EXPOSE 3000

ENTRYPOINT ["bun", "run", "index.js"]

# ── API target: Rust Axum API ──────────────────────────────────────────
FROM rust:1.85-bookworm AS builder

WORKDIR /build

# Cache dependencies — copy only Cargo files first and build a dummy crate.
COPY Cargo.toml Cargo.lock ./
COPY crates/titen-core/Cargo.toml crates/titen-core/
COPY crates/titen-api/Cargo.toml crates/titen-api/
COPY crates/titen-cli/Cargo.toml crates/titen-cli/
COPY crates/titen-mcp/Cargo.toml crates/titen-mcp/

RUN mkdir -p crates/titen-api/src && echo "fn main(){}" > crates/titen-api/src/main.rs && echo "" > crates/titen-api/src/lib.rs
RUN mkdir -p crates/titen-core/src && echo "" > crates/titen-core/src/lib.rs
RUN mkdir -p crates/titen-cli/src && echo "fn main(){}" > crates/titen-cli/src/main.rs
RUN mkdir -p crates/titen-mcp/src && echo "fn main(){}" > crates/titen-mcp/src/main.rs && echo "" > crates/titen-mcp/src/lib.rs

RUN cargo build --release -p titen-api -p titen-cli -p titen-mcp 2>/dev/null || true

# Copy real source and rebuild (cached deps reused).
COPY crates/ crates/
RUN touch crates/titen-api/src/main.rs crates/titen-api/src/lib.rs
RUN cargo build --release -p titen-api -p titen-cli -p titen-mcp

# ── API runtime ──────────────────────────────────────────────────────
FROM debian:bookworm-slim

ARG TARGETARCH
ARG VERSION=dev

LABEL version=${VERSION}
LABEL org.opencontainers.image.version=${VERSION}
LABEL org.opencontainers.image.description="Self-hosted Threads management platform"

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3t64 libstdc++6 curl && \
    rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd --system --gid 1000 titen && \
    useradd --system --uid 1000 --gid titen --home /data titen

# Copy Rust binaries
COPY --from=builder /build/target/release/titen-api /usr/local/bin/titen-api
COPY --from=builder /build/target/release/titen /usr/local/bin/titen
COPY --from=builder /build/target/release/titen-mcp /usr/local/bin/titen-mcp

# Data directory (mount volume here for persistence)
ENV TITEN_DB_PATH=/data/titen.db
ENV TITEN_HOST=0.0.0.0
ENV TITEN_PORT=7845

# Create data directory with correct ownership
RUN mkdir -p /data && chown titen:titen /data

USER titen

EXPOSE 7845

HEALTHCHECK --interval=30s --timeout=5s --retries=3 --start-period=10s \
    CMD ["curl", "-f", "http://localhost:7845/health"]

ENTRYPOINT ["titen-api"]
