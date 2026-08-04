# ── Stage 1: Web builder ────────────────────────────────────────────
FROM node:22-slim AS web-builder

WORKDIR /web

# Copy FE source
COPY web/ ./

# Install Bun and build
RUN npm install -g bun && \
    bun install --frozen-lockfile && \
    bun run build

# ── Stage 2: Binary builder ────────────────────────────────────────
FROM debian:trixie-slim AS builder

ARG TARGETARCH
ARG VERSION=dev

LABEL version=${VERSION}
LABEL org.opencontainers.image.version=${VERSION}

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy CI-downloaded binaries (preferred).
COPY binaries/ ./
RUN if [ "$TARGETARCH" = "arm64" ]; then \
      mv titen-api-arm64 titen-api && mv titen-arm64 titen && mv titen-mcp-arm64 titen-mcp && \
      rm -f titen-api-amd64 titen-amd64 titen-mcp-amd64; \
    else \
      mv titen-api-amd64 titen-api && mv titen-amd64 titen && mv titen-mcp-amd64 titen-mcp && \
      rm -f titen-api-arm64 titen-arm64 titen-mcp-arm64; \
    fi && \
    chmod +x titen-api titen titen-mcp

# ── Stage 3: Runtime ────────────────────────────────────────────────
FROM debian:trixie-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3t64 libstdc++6 curl && \
    rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd --system --gid 1000 titen && \
    useradd --system --uid 1000 --gid titen --home /data titen

# Copy binaries
COPY --from=builder /build/titen-api /usr/local/bin/titen-api
COPY --from=builder /build/titen /usr/local/bin/titen
COPY --from=builder /build/titen-mcp /usr/local/bin/titen-mcp

# Copy web dashboard build output
COPY --from=web-builder /web/build /app/web

# Data directory (mount volume here for persistence)
ENV TITEN_DB_PATH=/data/titen.db
ENV TITEN_HOST=0.0.0.0
ENV TITEN_PORT=7845
ENV TITEN_WEB_DIR=/app/web

# Create data directory with correct ownership
RUN mkdir -p /data && chown titen:titen /data

USER titen

EXPOSE 7845

ENTRYPOINT ["titen-api"]
