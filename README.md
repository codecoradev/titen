# titen

> **titen** (Javanese): memperhatikan, memantai, cermat mengawasi.

Self-hosted Threads management platform built with Rust.

## Features

- **Multi-account** token management with auto-refresh
- **Post scheduling** (Threads API has no native scheduling)
- **Comment fetching** and storage
- **Sentiment analysis** on comments (pluggable engine)
- **Analytics** tracking with time-series snapshots
- **Threads API proxy** with rate limiting
- **S3-compatible** media storage
- **HTTP API**, **CLI**, and **MCP** interfaces

## Quick Start

```bash
# Build
cargo build --release

# Start HTTP server
./target/release/titen-api

# CLI usage
./target/release/titen account add codecoradev 123456 "TOKEN" "2026-09-15T00:00:00Z"
./target/release/titen post create codecoradev --text "Hello from titen!"
./target/release/titen schedule add codecoradev --text "Scheduled" --at "2026-08-01T10:00:00+07:00"
```

## Configuration

All config via environment variables. See [DESIGN.md](DESIGN.md) for full details.

| Variable | Default | Description |
|----------|---------|-------------|
| `TITEN_PORT` | `7845` | HTTP server port |
| `TITEN_DB_PATH` | `./titen.db` | SQLite path |
| `TITEN_API_KEY` | (none) | API auth key (required for non-dev) |
| `TITEN_LOG_LEVEL` | `info` | Log level |

## Architecture

```
titen-core    → domain logic, SQLite store, traits
titen-api     → Axum HTTP server
titen-cli     → Clap CLI binary
titen-mcp     → MCP server (stdio JSON-RPC)
```

See [DESIGN.md](DESIGN.md) for the full architecture specification.

## License

AGPL-3.0-only
