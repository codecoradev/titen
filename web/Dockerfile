# ── Titen Web — SvelteKit (adapter-node) ──────────────────────────────────
# Built with bun, served with node (adapter-node target).

FROM oven/bun:1 AS builder

WORKDIR /app

# Cache deps first
COPY package.json bun.lock* package-lock.json* ./
RUN bun install --frozen-lockfile || bun install

COPY . .

# Build SvelteKit production output → /app/build
RUN bun run build

# ── Runtime ───────────────────────────────────────────────────────────────
FROM node:22-slim

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/build ./build

# adapter-node: binds 0.0.0.0:3000 by default
ENV HOSTNAME=0.0.0.0
ENV PORT=3000

EXPOSE 3000

CMD ["node", "build/index.js"]
