# Titen

> **titen** (Javanese): memperhatikan, memantai, cermat mengawasi.

Self-hosted Threads management platform — Rust + SQLite

---

## Features

- Multi-account management with token refresh
- Post creation and scheduling (Threads API has no native scheduling)
- Comment fetching and storage from the Threads Graph API
- Sentiment analysis on comments (pluggable keyword/stub engines, bilingual EN+ID)
- Analytics tracking with time-series snapshots
- Media upload to S3-compatible storage
- HTTP API with API key authentication
- CLI tool for all operations
- MCP server (stdio JSON-RPC) for AI agent integration

## Architecture

Titen is a Rust workspace with four crates:

| Crate | Purpose |
|---|---|
| `titen-core` | Domain logic — models, SQLite store, Threads API client, sentiment engine, scheduler, S3 storage |
| `titen-api` | Axum HTTP server — REST API with API key middleware, CORS, rate limiting |
| `titen-cli` | Clap CLI binary — all CRUD operations via the HTTP API |
| `titen-mcp` | MCP stdio server — JSON-RPC 2.0 tools for Claude Desktop, Cursor, and other MCP clients |

```
titen-core    → domain logic, SQLite store, traits
titen-api     → Axum HTTP server
titen-cli     → Clap CLI binary
titen-mcp     → MCP server (stdio JSON-RPC)
```

## Quick Start

### Prerequisites

- Rust 1.85+ (edition 2024)
- SQLite3 (bundled via sqlx)

### Build

```bash
git clone https://github.com/codecoradev/titen.git
cd titen
cargo build --release
```

### Run

```bash
# Start the HTTP server (default: 0.0.0.0:7845)
./target/release/titen-api

# Or via the CLI with embedded server
./target/release/titen serve

# Use the CLI against the running server
export TITEN_API_KEY=your-key
./target/release/titen account add myuser --access-token "THREADS_TOKEN" --expires-at "2026-12-01T00:00:00Z"
./target/release/titen post create myuser --text "Hello from titen!"
```

## Configuration

All configuration is via environment variables:

| Variable | Default | Description |
|---|---|---|
| `TITEN_DB_PATH` | `./titen.db` | SQLite database path |
| `TITEN_API_KEY` | *(none)* | API key for authentication (unset = open access) |
| `TITEN_HOST` | `0.0.0.0` | HTTP server bind address |
| `TITEN_PORT` | `7845` | HTTP server port |
| `TITEN_URL` | `http://localhost:7845` | Base URL for CLI (talking to API server) |
| `TITEN_SENTIMENT_ENGINE` | `keyword` | Sentiment engine: `keyword` or `stub` |
| `TITEN_SCHEDULER_INTERVAL_SECS` | `60` | Scheduler tick interval in seconds |
| `TITEN_S3_ENDPOINT` | *(none)* | S3-compatible storage endpoint URL |
| `TITEN_S3_BUCKET` | *(none)* | S3 bucket name |
| `TITEN_S3_REGION` | `us-east-1` | S3 region |
| `TITEN_S3_ACCESS_KEY` | *(none)* | S3 access key |
| `TITEN_S3_SECRET_KEY` | *(none)* | S3 secret key |
| `TITEN_S3_PUBLIC_URL` | *(none)* | Public URL for S3 objects (overrides endpoint) |

## CLI Usage

The CLI talks to the running HTTP server. Set `TITEN_URL` and `TITEN_API_KEY` as needed.

### Server

```bash
titen serve [--host 0.0.0.0] [--port 7845] [--mcp]
```

### Accounts

```bash
titen account list
titen account add <username> --access-token <TOKEN> [--user-id <ID>] [--expires-at <ISO8601>]
titen account remove <id>
titen account refresh <id>
titen account token-check <id>
titen token-check          # check all accounts
```

### Posts

```bash
titen post create <account> --text <TEXT> [--media-type TEXT|IMAGE] [--image-url <URL>]
titen post list [--account <id>] [--status <status>]
titen post delete <post_id>
titen post insights <post_id>
```

### Schedules

```bash
titen schedule create <account> --text <TEXT> --at <ISO8601> [--media-type TEXT|IMAGE]
titen schedule list [--account <id>] [--status <status>]
titen schedule cancel <id>
titen schedule upcoming
```

### Comments

```bash
titen comment fetch <post_id>     # fetch from Threads API
titen comment list <post_id>      # list stored comments
titen comment sentiment <post_id> # analyze sentiment
```

### Analytics

```bash
titen analytics posts <account> [--from <date>] [--to <date>]
titen analytics trend <post_id>
```

### Media

```bash
titen media list
titen media upload <file_path> [--content-type <mime>]
titen media delete <id>
```

## API Reference

Base URL: `http://localhost:7845`

All endpoints (except `/health`) require the `X-API-Key` header when `TITEN_API_KEY` is set:

```
X-API-Key: your-key-here
```

### Health

| Method | Path | Description |
|---|---|---|
| GET | `/health` | Health check (no auth required) |

### Accounts

| Method | Path | Description |
|---|---|---|
| GET | `/api/accounts` | List all accounts |
| POST | `/api/accounts` | Create an account |
| PUT | `/api/accounts/{id}` | Update an account |
| DELETE | `/api/accounts/{id}` | Delete an account |
| POST | `/api/accounts/{id}/refresh-token` | Refresh account token |

### Posts

| Method | Path | Description |
|---|---|---|
| GET | `/api/posts` | List posts (query: `account_id`, `status`, `limit`, `offset`) |
| POST | `/api/posts` | Create and publish a post |
| GET | `/api/posts/{id}` | Get a single post |
| DELETE | `/api/posts/{id}` | Delete a post |
| GET | `/api/posts/{id}/insights` | Fetch and store post insights |

### Schedules

| Method | Path | Description |
|---|---|---|
| GET | `/api/schedules` | List schedules (query: `account_id`, `status`) |
| POST | `/api/schedules` | Create a schedule |
| PUT | `/api/schedules/{id}` | Update a schedule |
| DELETE | `/api/schedules/{id}` | Delete a schedule |
| GET | `/api/schedules/upcoming` | List next 10 pending schedules |

### Comments

| Method | Path | Description |
|---|---|---|
| GET | `/api/posts/{id}/comments` | List stored comments |
| POST | `/api/posts/{id}/comments/fetch` | Fetch comments from Threads API |
| GET | `/api/posts/{id}/comments/sentiment` | Analyze comment sentiment |

### Analytics

| Method | Path | Description |
|---|---|---|
| GET | `/api/analytics/posts` | Account post analytics (query: `account_id`, `from`, `to`) |
| GET | `/api/analytics/posts/{id}/trend` | Time-series trend for a post |

### Media

| Method | Path | Description |
|---|---|---|
| GET | `/api/media` | List uploaded media |
| POST | `/api/media` | Upload media (multipart form) |
| DELETE | `/api/media/{id}` | Delete media |

## MCP Server

Titen includes an MCP (Model Context Protocol) server that exposes tools for AI agents. It communicates over stdio using JSON-RPC 2.0.

### Configuration

**Claude Desktop** (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "titen": {
      "command": "/path/to/titen-mcp",
      "env": {
        "TITEN_DB_PATH": "/path/to/titen.db"
      }
    }
  }
}
```

**Cursor** — add to your MCP settings:

```json
{
  "mcp": {
    "titen": {
      "command": "/path/to/titen-mcp",
      "env": {
        "TITEN_DB_PATH": "/path/to/titen.db"
      }
    }
  }
}
```

### Available Tools

| Tool | Description |
|---|---|
| `list_accounts` | List all Threads accounts |
| `create_post` | Create and publish a post (args: `account_id`, `caption`, `media_type`, `image_url`) |
| `schedule_post` | Schedule a post (args: `account_id`, `caption`, `scheduled_at`, `media_type`) |
| `list_schedules` | List scheduled posts (args: `account_id`, `status`) |
| `cancel_schedule` | Cancel a scheduled post (args: `id`) |
| `fetch_comments` | Fetch and store comments from a post (args: `post_id`) |
| `get_post_sentiment` | Get sentiment analysis for a post's comments (args: `post_id`) |
| `get_account_analytics` | Get analytics for an account (args: `account_id`) |
| `delete_post` | Delete a post (args: `post_id`) |
| `check_tokens` | Check all accounts' token expiry status |

## Docker

```dockerfile
FROM rust:1.85-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/titen-api /usr/local/bin/
COPY --from=builder /app/target/release/titen-cli /usr/local/bin/
COPY --from=builder /app/target/release/titen-mcp /usr/local/bin/
EXPOSE 7845
CMD ["titen-api"]
```

Build and run:

```bash
docker build -t titen .
docker run -p 7845:7845 \
  -e TITEN_API_KEY=your-key \
  -e TITEN_S3_ENDPOINT=https://s3.example.com \
  -e TITEN_S3_BUCKET=titen-media \
  -v titen-data:/data \
  titen -e TITEN_DB_PATH=/data/titen.db
```

## License

AGPL-3.0-only

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Commit your changes
4. Push to the branch (`git push origin feature/my-feature`)
5. Open a Pull Request
