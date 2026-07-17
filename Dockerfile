# ── Build stage ──────────────────────────────────────────────
FROM rust:1.85-bookworm AS builder

WORKDIR /app

# Cache deps
COPY Cargo.toml Cargo.lock ./
COPY crates/titen-core/Cargo.toml crates/titen-core/
COPY crates/titen-api/Cargo.toml crates/titen-api/
COPY crates/titen-cli/Cargo.toml crates/titen-cli/
COPY crates/titen-mcp/Cargo.toml crates/titen-mcp/
RUN mkdir -p crates/titen-core/src && echo "" > crates/titen-core/src/lib.rs
RUN mkdir -p crates/titen-api/src && echo "" > crates/titen-api/src/lib.rs && echo "fn main(){}" > crates/titen-api/src/main.rs
RUN mkdir -p crates/titen-cli/src && echo "fn main(){}" > crates/titen-cli/src/main.rs
RUN mkdir -p crates/titen-mcp/src && echo "fn main(){}" > crates/titen-mcp/src/main.rs
RUN cargo build --release 2>/dev/null || true

# Real build
COPY . .
RUN touch crates/titen-core/src/lib.rs crates/titen-api/src/lib.rs crates/titen-api/src/main.rs
RUN touch crates/titen-cli/src/main.rs crates/titen-mcp/src/main.rs
RUN cargo build --release --bin titen-api --bin titen-cli --bin titen-mcp

# ── Runtime stage ──────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/titen-api /usr/local/bin/titen-api
COPY --from=builder /app/target/release/titen-cli /usr/local/bin/titen-cli
COPY --from=builder /app/target/release/titen-mcp /usr/local/bin/titen-mcp

# Data directory (DB + config)
RUN mkdir -p /data/titen
ENV TITEN_DB_PATH=/data/titen/titen.db
ENV TITEN_HOST=0.0.0.0
ENV TITEN_PORT=7845

EXPOSE 7845

WORKDIR /data/titen
ENTRYPOINT ["titen-api"]
