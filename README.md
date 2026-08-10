<!-- Badges -->
[![CI](https://github.com/codecoradev/titen/actions/workflows/ci.yml/badge.svg)](https://github.com/codecoradev/titen/actions/workflows/ci.yml)
[![Release](https://github.com/codecoradev/titen/actions/workflows/release.yml/badge.svg)](https://github.com/codecoradev/titen/actions/workflows/release.yml)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![Rust 1.85+](https://img.shields.io/badge/Rust-1.85+-orange.svg)](https://www.rust-lang.org/)

# Titen

> *titen* (Javanese): to watch closely, to observe with care.

Self-hosted Threads management platform. Post, schedule, and analyze Threads content from your own infrastructure.

## Why Titen?

Threads has no native post scheduling. Existing tools are SaaS products where your API tokens live on someone else's server. Titen runs on your box, talks directly to the Threads Graph API, and stores everything in a single SQLite file.

No subscription. No vendor lock-in. Your tokens stay on your machine, encrypted at rest with AES-256-GCM.

## Features

| Capability | Details |
|---|---|
| **Multi-account** | Manage multiple Threads accounts in one instance |
| **Post scheduling** | Cron-based scheduler for automated posting that Threads itself does not offer |
| **Comment fetching** | Pull comments from the Threads API, store locally |
| **Sentiment analysis** | Pluggable engine trait (stub default; ONNX/LLM/custom API extensible) |
| **Analytics** | Time-series snapshots per post |
| **Media storage** | S3-compatible via swappable storage trait |
| **MCP server** | JSON-RPC 2.0 over stdio, compatible with Claude Desktop, Cursor, etc. |
| **CLI** | Full CRUD from the terminal |
| **Docker** | Single container, minimal footprint |

## Architecture

> **Detailed docs:** [Deployment Guide](docs/deployment.md) · [Usage Guide](docs/usage.md) · [Architecture Overview](docs/architecture.md) · [Auth Flow](docs/auth-flow.md) · [Changelog](CHANGELOG.md)

4 crates, one binary each:

| Crate | Purpose |
|---|---|
| `titen-core` | Domain logic: models, SQLite store, Threads API client, sentiment trait, scheduler, S3 storage, AES-256-GCM encryption |
| `titen-api` | Axum HTTP server: REST API, API key auth, CORS, rate limiting |
| `titen-cli` | Clap CLI: all operations via the HTTP API |
| `titen-mcp` | MCP stdio server: 17 tools for AI agent integration |

8 SQLite tables: `accounts`, `posts`, `schedules`, `comments`, `analytics_snap`, `media_assets`, `rate_tracking`, `_encryption_meta`.

4 migrations: `001_initial` (schema), `002_drop_refresh_token`, `003_add_app_secret`, `004_encrypt_tokens` (encrypts existing plaintext tokens on startup).

## Quick Start

### Install (pre-built binary)

```bash
curl -sSL https://github.com/codecoradev/titen/releases/latest/download/install.sh | sh
```

### Build from source

Requires Rust 1.85+ (edition 2024). SQLite is bundled via sqlx.

```bash
git clone https://github.com/codecoradev/titen.git
cd titen
cargo build --release
```

### Run

```bash
# Start the API server (default: 0.0.0.0:7845)
titen-api

# Or via the CLI with embedded server
titen serve

# Add an account and post
export TITEN_API_KEY=your-key
titen account add myuser --access-token "THREADS_TOKEN" --expires-at "2026-12-01T00:00:00Z"
titen post create myuser --text "Hello from titen!"
```

The server creates a SQLite database at `~/.codecora/titen/titen.db` by default (override with `TITEN_DB_PATH`).

## Configuration

All config via environment variables:

| Variable | Default | Description |
|---|---|---|
| `TITEN_DB_PATH` | `~/.codecora/titen/titen.db` | SQLite database path |
| `TITEN_API_KEY` | *(none)* | API key for endpoint access. When unset, all endpoints are open (dev mode) |
| `TITEN_ENCRYPTION_KEY` | *(none)* | AES-256-GCM key for token encryption at rest. Generate with `openssl rand -hex 32` |
| `TITEN_REQUIRE_ENCRYPTION` | `false` | Set to `true` in production to fail-fast if encryption key is missing |
| `TITEN_HOST` | `0.0.0.0` | Bind address |
| `TITEN_PORT` | `7845` | Bind port |
| `TITEN_URL` | `http://localhost:7845` | Base URL for CLI |
| `TITEN_SENTIMENT_ENGINE` | `stub` | `stub`, `onnx`, `llm`, or `custom_api` |
| `TITEN_SCHEDULER_INTERVAL_SECS` | `60` | Scheduler tick interval |
| `TITEN_S3_ENDPOINT` | *(none)* | S3-compatible endpoint |
| `TITEN_S3_BUCKET` | *(none)* | S3 bucket name |
| `TITEN_S3_REGION` | `us-east-1` | S3 region |
| `TITEN_S3_ACCESS_KEY` | *(none)* | S3 access key |
| `TITEN_S3_SECRET_KEY` | *(none)* | S3 secret key |
| `TITEN_S3_PUBLIC_URL` | *(none)* | Public URL for uploaded media |

## API Reference

Base URL: `http://localhost:7845`

**Interactive docs (Swagger UI):** `http://localhost:7845/api/docs` — explore and test all endpoints directly from the browser.

**OpenAPI JSON:** `http://localhost:7845/api/docs/openapi.json`

All endpoints except `/health` and `/api/docs` require `X-API-Key` authentication when `TITEN_API_KEY` is set. Use a plain header (not Bearer):

```
X-API-Key: your-key-here
```

### Health

| Method | Path | Auth | Description |
|---|---|---|---|
| GET | `/health` | None | Server health check |

### Accounts

| Method | Path | Description | Query Params |
|---|---|---|---|
| GET | `/api/accounts` | List all accounts | — |
| POST | `/api/accounts` | Create an account | — |
| PUT | `/api/accounts/{id}` | Update an account | — |
| DELETE | `/api/accounts/{id}` | Delete an account | — |
| POST | `/api/accounts/{id}/refresh-token` | Refresh OAuth token | — |
| GET | `/api/accounts/{id}/profile` | Fetch Threads profile (`/me`) | — |
| GET | `/api/accounts/{id}/publishing-limit` | Get remaining daily limits | — |
| GET | `/api/accounts/{id}/insights` | Account-level insights (followers, media count) | — |
| GET | `/api/accounts/check-tokens` | Batch token expiry check + auto-refresh | — |

### Posts

| Method | Path | Description | Query Params |
|---|---|---|---|
| GET | `/api/posts` | List posts | `?account_id=&status=&limit=&offset=` |
| POST | `/api/posts` | Create and publish a post | — |
| GET | `/api/posts/{id}` | Get a single post | — |
| DELETE | `/api/posts/{id}` | Delete a post | — |
| GET | `/api/posts/{id}/insights` | Fetch and store post insights | — |

### Schedules

| Method | Path | Description | Query Params |
|---|---|---|---|
| GET | `/api/schedules` | List schedules | `?account_id=&status=` |
| POST | `/api/schedules` | Create a schedule | — |
| GET | `/api/schedules/{id}` | Get a single schedule | — |
| PUT | `/api/schedules/{id}` | Full update (all fields) | — |
| PATCH | `/api/schedules/{id}` | Partial update (specific fields) | — |
| DELETE | `/api/schedules/{id}` | Delete a schedule | — |
| POST | `/api/schedules/{id}/approve` | Approve draft → pending (HITL) | — |
| POST | `/api/schedules/{id}/reject` | Reject draft (HITL) | — |
| GET | `/api/schedules/upcoming` | Next 10 pending schedules | — |

### Comments

| Method | Path | Description | Query Params |
|---|---|---|---|
| GET | `/api/posts/{id}/comments` | List stored comments | — |
| POST | `/api/posts/{id}/comments/fetch` | Fetch comments from Threads API | — |
| GET | `/api/posts/{id}/comments/sentiment` | Analyze comment sentiment | — |

### Analytics

| Method | Path | Description | Query Params |
|---|---|---|---|
| GET | `/api/analytics/posts` | Post analytics summary | `?account_id=&from=&to=` |
| GET | `/api/analytics/posts/{id}/trend` | Time-series engagement trend | — |

### Media

| Method | Path | Description |
|---|---|---|
| GET | `/api/media` | List uploaded media |
| POST | `/api/media` | Upload media (multipart `file` field) |
| GET | `/api/media/{id}` | Get a single media asset |
| DELETE | `/api/media/{id}` | Delete media |

### Threads (low-level passthrough)

| Method | Path | Description |
|---|---|---|
| POST | `/api/threads/container` | Create a Threads media container |
| POST | `/api/threads/container/{id}/publish` | Publish a container |
| POST | `/api/threads/container/{id}/status` | Check container publishing status |
| POST | `/api/threads/reply` | Reply to a Threads post or comment |
| POST | `/api/threads/reply/{id}/hide` | Hide/unhide a reply |
| GET | `/api/threads/profile-lookup` | Look up any Threads user profile |
| POST | `/api/threads/search` | Search Threads by keyword |
| POST | `/api/threads/mentions` | Fetch mentions for an account |
| POST | `/api/threads/share-to-instagram` | Cross-post to Instagram |

### Authentication (web UI session)

| Method | Path | Description |
|---|---|---|
| POST | `/api/auth/login` | Login (password → session cookie) |
| GET | `/api/auth/session` | Check current session |
| POST | `/api/auth/logout` | Logout (clear session) |

### OAuth

| Method | Path | Description |
|---|---|---|
| POST | `/api/oauth/exchange` | Exchange Threads OAuth code for access token |

### Request/Response Format

All responses use JSON with a `data` field for successful requests:

```json
{ "data": { ... } }
```

Or `data` array for list endpoints:

```json
{ "data": [ ... ], "count": 42 }
```

Errors return a JSON body with `error` and `code`:

```json
{ "error": "Schedule not found", "code": "NOT_FOUND" }
```

### Key Data Models

**Schedule** — created as `draft`, requires approval before auto-publishing:

```json
{
  "id": "019fdfae-dcad-7093-95d1-236065ad8aff",
  "account_id": "019fdfae-c0b1-7031-afad-e7ae3ed80646",
  "caption": "Post text with #hashtags",
  "media_type": "CAROUSEL",
  "media_urls": ["https://cdn.example.com/slide-01.jpg", "https://cdn.example.com/slide-02.jpg"],
  "scheduled_at": "2026-08-09T12:00:00+07:00",
  "status": "draft",
  "approved_at": null,
  "created_at": "2026-08-08T10:00:00Z"
}
```

**Schedule lifecycle (HITL flow):**

```
draft → (approve) → pending → (scheduler at scheduled_at) → published
draft → (reject)  → rejected
pending → (publish fails) → failed
```

Only `pending` schedules are picked up by the scheduler. A schedule stays as `draft` until explicitly approved via `POST /api/schedules/{id}/approve`.

**Media types supported:** `TEXT`, `IMAGE`, `CAROUSEL`, `VIDEO`

For `IMAGE` and `CAROUSEL`, `media_urls` must contain publicly accessible URLs to hosted images. Titen does not download or re-host images referenced in schedules (use `POST /api/media` to upload first if needed).

## CLI

The CLI talks to the running HTTP server. Set `TITEN_URL` and `TITEN_API_KEY` as needed.

```bash
titen serve [--host 0.0.0.0] [--port 7845] [--mcp]
```

### Accounts

```bash
titen account list
titen account add <username> --access-token <TOKEN> [--user-id <ID>] [--expires-at <ISO8601>]
titen account remove <id>
titen account refresh <id>
titen account status <id>
titen token-check
```

### Posts

```bash
titen post create <account> --text <TEXT> [--media-type TEXT|IMAGE] [--image-url <URL>]
titen post delete <post_id>
titen post insights <post_id>
```

### Schedules

```bash
titen schedule add <account> --text <TEXT> --at <ISO8601> [--media-type TEXT|IMAGE]
titen schedule list [--account <id>] [--status <status>]
titen schedule cancel <id>
titen schedule upcoming
```

### Comments

```bash
titen comment fetch <post_id>
titen comment list <post_id>
titen comment sentiment <post_id>
```

### Analytics

```bash
titen analytics posts <account> [--from <date>] [--to <date>]
titen analytics trend <post_id>
titen analytics sentiment-summary <post_id>
```

### Media

```bash
titen media list
titen media upload <file_path> [--content-type <mime>]
titen media delete <id>
```

## MCP Server

Titen ships an MCP (Model Context Protocol) server for AI agent integration. It communicates over stdio using JSON-RPC 2.0.

### Setup

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

**Cursor**: add to your MCP settings:

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

### Available Tools (17)

| Tool | Description | Key Parameters |
|---|---|---|
| `list_accounts` | List all Threads accounts | — |
| `get_user_profile` | Fetch a Threads user's profile | `account_id` |
| `get_publishing_limit` | Fetch daily publishing quota | `account_id` |
| `create_post` | Create and publish a post | `account_id`, `caption`, `media_type` |
| `schedule_post` | Schedule a post for future publishing | `account_id`, `caption`, `scheduled_at` |
| `list_schedules` | List scheduled posts | `account_id?`, `status?` |
| `cancel_schedule` | Cancel a scheduled post | `id` |
| `refresh_token` | Refresh an account's access token | `account_id` |
| `check_tokens` | Batch check all token expiry + auto-refresh | — |
| `fetch_comments` | Fetch and store comments from Threads API | `post_id` |
| `get_post_sentiment` | Sentiment analysis for a post's comments | `post_id` |
| `get_post_insights` | Fetch post engagement metrics | `post_id` |
| `get_account_analytics` | Analytics summary for an account | `account_id` |
| `delete_post` | Delete a post from Threads + DB | `post_id` |
| `create_container` | Create a Threads media container | `account_id`, `media_type` |
| `publish_container` | Publish a previously created container | `account_id`, `container_id` |

> **Note:** 13 additional API endpoints do not yet have MCP tool wrappers (list_posts, get_schedule, approve_schedule, upload_media, search, mentions, reply, etc.). See [issue #84](https://github.com/codecoradev/titen/issues/84) for the tracking issue.

## Threads API Limits

The platform enforces these per-account daily limits:

| Limit | Count |
|---|---|
| Posts | 250/day |
| Replies | 1,000/day |
| Deletes | 100/day |
| Caption length | 500 chars |
| `text_attachment` length | 10,000 chars |

## Scheduler Behavior

- Engine: tokio-cron with a 60-second tick interval
- Flow: at scheduled time, create media container, wait for ready, publish
- Publish delays: text posts publish immediately (0s), image posts wait 30s, video posts wait 60s for processing

## Security

- **Token encryption**: `access_token` and `app_secret` columns encrypted at rest with AES-256-GCM. Each value gets a random 96-bit nonce and a `enc:v1:` versioned prefix for future migration. Key is zeroized on drop.
- **Fail-fast mode**: set `TITEN_REQUIRE_ENCRYPTION=true` in production to reject startup if the encryption key is missing.
- **API key auth**: constant-time comparison against `TITEN_API_KEY`. Three credential sources: `X-API-Key` header, `api_key` query param, `titen_session` cookie.
- **HTTP client timeouts**: all outbound calls to the Threads API and S3 have connect and total timeout limits.

See [SECURITY.md](SECURITY.md) for the full policy and vulnerability reporting.

## Docker

```bash
docker build -t titen .
docker run -p 7845:7845 \
  -e TITEN_API_KEY=your-key \
  -e TITEN_DB_PATH=/data/titen.db \
  -e TITEN_S3_ENDPOINT=https://s3.example.com \
  -e TITEN_S3_BUCKET=titen-media \
  -v titen-data:/data \
  titen
```

## License

[Apache-2.0](LICENSE)

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Commit your changes
4. Push to the branch (`git push origin feature/my-feature`)
5. Open a Pull Request