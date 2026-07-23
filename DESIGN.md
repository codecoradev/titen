# titen — Self-Hosted Threads Management Platform

> **titen** (Javanese): memperhatikan, memantai, cermat mengawasi.

Self-hosted backend for managing Threads accounts, scheduling posts, scraping comments, analytics, and sentiment analysis. Built with Rust (Axum + SQLx + SQLite), deployed via Docker.

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Crate Layout](#crate-layout)
4. [Data Model (SQLite Schema)](#data-model)
5. [API Specification](#api-specification)
6. [CLI Specification](#cli-specification)
7. [MCP Server Specification](#mcp-server-specification)
8. [Threads API Proxy](#threads-api-proxy)
9. [Scheduler](#scheduler)
10. [S3 / Media Management](#s3--media-management)
11. [Sentiment Analysis](#sentiment-analysis)
12. [Token Lifecycle Management](#token-lifecycle-management)
13. [Configuration (ENV)](#configuration-env)
14. [Docker Deployment](#docker-deployment)
15. [Development Roadmap](#development-roadmap)
16. [Naming Conventions](#naming-conventions)

---

## Overview

### Problem

Managing multiple Threads accounts requires manual token tracking, no native scheduling (Threads API has no `publish_at`), no unified analytics dashboard, and no sentiment analysis tooling. Existing solutions (Repliz, Buffer) are SaaS and don't offer self-hosted control.

### Solution

`titen` is a single self-hosted binary that:

- **Manages multiple Threads accounts** with automatic token refresh
- **Schedules posts** via a local scheduler (SQLite-backed, no external dependencies)
- **Fetches and stores comments** on owned posts for analysis
- **Tracks analytics** (likes, replies, reposts, views) over time
- **Runs sentiment analysis** on comments (pluggable engine: stub → ONNX → LLM API)
- **Proxies official Threads API** with rate limiting
- **Stores media assets** on any S3-compatible storage
- **Exposes HTTP API, CLI, and MCP** from the same core

### Constraints

- **Threads API limitations**: No `publish_at` (scheduling built locally), no likes/follows/search via API, no competitor content fetching, max 5 URLs per post, 250 posts/24h, container expiry 24h.
- **Rate limits**: 250 posts/day, 1000 replies/day, 100 deletes/day per account.
- **Text limits**: 500 chars (main), 10,000 chars (text_attachment).

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         titen                                │
│                                                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────────┐  │
│  │ HTTP API │  │   CLI    │  │   MCP    │  │  Scheduler  │  │
│  │  (Axum)  │  │  (clap)  │  │ (stdio)  │  │ (tokio-cron)│  │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └──────┬─────┘  │
│       │             │              │                 │        │
│  ─────┴─────────────┴──────────────┴─────────────────┴────── │
│                      Core Domain                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                    titen-core                            │  │
│  │  ┌─────────┐ ┌─────────┐ ┌──────────┐ ┌───────────┐  │  │
│  │  │Accounts │ │  Posts  │ │ Comments │ │Analytics  │  │  │
│  │  │+Tokens  │ │Schedule │ │+Scrape   │ │+Insights  │  │  │
│  │  └─────────┘ └─────────┘ └──────────┘ └───────────┘  │  │
│  │  ┌─────────────┐ ┌──────────┐ ┌────────────────────┐  │  │
│  │  │  Sentiment  │ │   S3     │ │  Threads Proxy    │  │  │
│  │  │  Engine     │ │  Storage │ │  + Rate Limiter   │  │  │
│  │  └─────────────┘ └──────────┘ └────────────────────┘  │  │
│  └──────────────────────────────────────────────────────┘  │
│                           │                                 │
│  ─────────────────────────┼──────────────────────────────── │
│              Storage Layer │                                  │
│  ┌───────────────┐  ┌─────────────────────────────────┐   │
│  │    SQLite     │  │     S3 Compatible (MinIO/R2/etc) │   │
│  │  accounts     │  │     media assets                  │   │
│  │  posts        │  │     exports                        │   │
│  │  comments     │  │     backups                        │   │
│  │  schedules    │  └─────────────────────────────────┘   │
│  │  analytics    │                                         │
│  │  sentiment    │         ┌─────────────────────────┐    │
│  │  rate_limits  │────────►│  Threads Graph API      │    │
│  └───────────────┘         │  (graph.threads.net)     │    │
│                             └─────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

### Design Principles

1. **SQLite-first, single binary** — zero external runtime dependencies except optional S3
2. **All interfaces from one core** — HTTP, CLI, MCP share the same domain logic via `titen-core`
3. **Pluggable sentiment** — trait-based, default stub, swappable at runtime via config
4. **Proxy-aware** — can act as transparent proxy or full management layer
5. **ENV-driven config** — no config files, everything via environment variables
6. **Docker-ready** — single Dockerfile, multi-stage build

---

## Crate Layout

```
titen/
├── Cargo.toml                    # workspace root
├── CLAUDE.md                     # project conventions (for AI coding agents)
├── DESIGN.md                     # this file
├── .github/
│   └── workflows/
│       ├── ci.yml                # fmt, clippy, test
│       └── release.yml           # cross-compile + Docker
├── Dockerfile                    # cargo-chef multi-stage
├── crates/
│   ├── titen-core/               # domain logic, Store, traits
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── store.rs          # SQLite store (all CRUD)
│   │       ├── account.rs        # account + token management
│   │       ├── post.rs           # post creation, publishing
│   │       ├── schedule.rs       # scheduling logic
│   │       ├── comment.rs        # comment fetching + storage
│   │       ├── analytics.rs      # insights aggregation
│   │       ├── sentiment.rs      # sentiment trait + stub impl
│   │       ├── proxy.rs          # Threads API proxy + rate limit
│   │       ├── storage.rs        # S3 client abstraction
│   │       └── error.rs          # unified error types
│   ├── titen-api/                # HTTP server (Axum)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── server.rs         # router + handlers
│   │   │   ├── routes/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── accounts.rs
│   │   │   │   ├── posts.rs
│   │   │   │   ├── schedules.rs
│   │   │   │   ├── comments.rs
│   │   │   │   ├── analytics.rs
│   │   │   │   ├── sentiment.rs
│   │   │   │   ├── media.rs
│   │   │   │   └── proxy.rs
│   │   │   ├── middleware.rs
│   │   │   └── config.rs
│   │   └── migrations/
│   │       └── 001_initial.sql
│   ├── titen-cli/                # CLI binary (clap)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs           # clap subcommands
│   └── titen-mcp/                # MCP server (stdio JSON-RPC)
│       ├── Cargo.toml
│       └── src/
│           └── main.rs
└── tests/
    └── integration/
        └── api_test.rs
```

### Workspace Cargo.toml

```toml
[workspace]
members = ["crates/*"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.88"
license = "AGPL-3.0-only"

[workspace.dependencies]
titen-core = { path = "crates/titen-core" }
axum = "0.8"
clap = { version = "4", features = ["derive"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
reqwest = { version = "0.12", features = ["json", "form"] }
anyhow = "1"
thiserror = "2"
tracing = "0.1"
tracing-subscriber = "0.3"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v7", "serde"] }
rust-s3 = "0.35"                  # S3 compatible client
tokio-cron-scheduler = "0.13"     # scheduler
```

---

## Data Model

### ER Diagram

```
┌──────────────────┐       ┌──────────────────┐
│     accounts      │       │     posts         │
├──────────────────┤       ├──────────────────┤
│ id  TEXT PK       │──────<│ id  TEXT PK       │
│ username TEXT UNQ │  1:N │ threads_post_id   │
│ user_id TEXT NOT N │       │ account_id FK     │
│ access_token TEXT  │       │ media_type TEXT   │
│ refresh_token TEXT │       │ caption TEXT      │
│ expires_at TEXT    │       │ text_attachment   │
│ app_id TEXT        │       │ carousel_children  │
│ is_active INTEGER  │       │ status TEXT       │
│ created_at TEXT    │       │ published_at TEXT  │
│ updated_at TEXT    │       │ insights_json TEXT │
└──────────────────┘       │ created_at TEXT    │
                           └────────┬─────────┘
                                    │ 1:N
┌──────────────────┐       ┌────────┴─────────┐
│   rate_tracking  │       │    comments       │
├──────────────────┤       ├──────────────────┤
│ id  TEXT PK       │       │ id  TEXT PK       │
│ account_id FK     │       │ post_id FK        │
│ action_type TEXT  │       │ threads_comment_id│
│ timestamp TEXT    │       │ author_username   │
│ count INTEGER     │       │ text TEXT         │
└──────────────────┘       │ sentiment TEXT    │
                           │ sentiment_score   │
┌──────────────────┐       │ fetched_at TEXT   │
│   schedules       │       └──────────────────┘
├──────────────────┤
│ id  TEXT PK       │       ┌──────────────────┐
│ account_id FK     │       │  analytics_snap   │
│ media_type TEXT   │       ├──────────────────┤
│ caption TEXT      │       │ id  TEXT PK       │
│ text_attachment   │──────<│ post_id FK        │
│ media_urls JSON   │  1:1  │ likes INTEGER     │
│ scheduled_at TEXT │       │ replies INTEGER   │
│ status TEXT       │       │ reposts INTEGER   │
│ published_at TEXT │       │ views INTEGER     │
│ result_json TEXT  │       │ quotes INTEGER    │
│ created_at TEXT   │       │ snapshot_at TEXT  │
│ error TEXT        │       └──────────────────┘
└──────────────────┘
                           ┌──────────────────┐
                           │  media_assets     │
                           ├──────────────────┤
                           │ id  TEXT PK       │
                           │ filename TEXT      │
                           │ content_type TEXT │
                           │ size INTEGER      │
                           │ s3_key TEXT       │
                           │ s3_url TEXT       │
                           │ uploaded_at TEXT  │
                           └──────────────────┘
```

### SQLite Schema

```sql
-- 001_initial.sql

-- Accounts (multi-account token management)
CREATE TABLE IF NOT EXISTS accounts (
    id            TEXT PRIMARY KEY,            -- UUID v7
    username      TEXT NOT NULL UNIQUE,
    user_id       TEXT NOT NULL,               -- Threads numeric user ID
    access_token  TEXT NOT NULL,
    refresh_token TEXT,
    expires_at    TEXT NOT NULL,               -- ISO 8601 datetime
    app_id        TEXT,
    is_active     INTEGER NOT NULL DEFAULT 1,
    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at    TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_accounts_username ON accounts(username);
CREATE INDEX idx_accounts_active ON accounts(is_active);

-- Posts (published posts tracking)
CREATE TABLE IF NOT EXISTS posts (
    id               TEXT PRIMARY KEY,          -- UUID v7
    threads_post_id  TEXT UNIQUE,
    account_id       TEXT NOT NULL REFERENCES accounts(id),
    media_type       TEXT NOT NULL DEFAULT 'TEXT',  -- TEXT|IMAGE|VIDEO|CAROUSEL
    caption          TEXT,
    text_attachment  TEXT,                       -- long-form text (up to 10k chars)
    carousel_children TEXT,                      -- JSON array of media IDs
    status           TEXT NOT NULL DEFAULT 'draft',  -- draft|scheduled|publishing|published|failed|expired
    scheduled_id     TEXT REFERENCES schedules(id),
    published_at     TEXT,
    insights_json    TEXT,                       -- cached insights JSON
    created_at       TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at       TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_posts_account ON posts(account_id);
CREATE INDEX idx_posts_status ON posts(status);
CREATE INDEX idx_posts_scheduled ON posts(scheduled_id);

-- Schedules (pending/scheduled posts)
CREATE TABLE IF NOT EXISTS schedules (
    id              TEXT PRIMARY KEY,           -- UUID v7
    account_id      TEXT NOT NULL REFERENCES accounts(id),
    media_type      TEXT NOT NULL DEFAULT 'TEXT',
    caption         TEXT,
    text_attachment  TEXT,
    media_urls      TEXT,                        -- JSON array of S3 URLs or external URLs
    scheduled_at    TEXT NOT NULL,               -- ISO 8601 datetime (when to publish)
    status          TEXT NOT NULL DEFAULT 'pending',  -- pending|processing|published|failed
    published_at    TEXT,
    result_post_id  TEXT,
    result_json     TEXT,                        -- full Threads API response
    error           TEXT,                        -- error message if failed
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_schedules_account ON schedules(account_id);
CREATE INDEX idx_schedules_status ON schedules(status);
CREATE INDEX idx_schedules_time ON schedules(scheduled_at);

-- Comments (fetched from owned posts)
CREATE TABLE IF NOT EXISTS comments (
    id                 TEXT PRIMARY KEY,         -- UUID v7
    post_id            TEXT NOT NULL REFERENCES posts(id),
    threads_comment_id TEXT,
    author_username    TEXT,
    author_user_id     TEXT,
    text               TEXT NOT NULL,
    sentiment          TEXT,                     -- positive|negative|neutral|null
    sentiment_score    REAL,                      -- -1.0 to 1.0
    fetched_at         TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_comments_post ON comments(post_id);
CREATE INDEX idx_comments_sentiment ON comments(sentiment);

-- Analytics snapshots (time-series per post)
CREATE TABLE IF NOT EXISTS analytics_snap (
    id           TEXT PRIMARY KEY,              -- UUID v7
    post_id      TEXT NOT NULL REFERENCES posts(id),
    likes        INTEGER NOT NULL DEFAULT 0,
    replies      INTEGER NOT NULL DEFAULT 0,
    reposts      INTEGER NOT NULL DEFAULT 0,
    views        INTEGER NOT NULL DEFAULT 0,
    quotes       INTEGER NOT NULL DEFAULT 0,
    snapshot_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_analytics_post ON analytics_snap(post_id);
CREATE INDEX idx_analytics_time ON analytics_snap(snapshot_at);

-- Media assets (S3-managed files)
CREATE TABLE IF NOT EXISTS media_assets (
    id           TEXT PRIMARY KEY,              -- UUID v7
    filename     TEXT NOT NULL,
    content_type TEXT NOT NULL,
    size_bytes   INTEGER NOT NULL,
    s3_key       TEXT NOT NULL,                 -- object key in bucket
    s3_url       TEXT,                          -- presigned or public URL
    uploaded_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_media_filename ON media_assets(filename);

-- Rate limit tracking (per account, sliding window)
CREATE TABLE IF NOT EXISTS rate_tracking (
    id           TEXT PRIMARY KEY,              -- UUID v7
    account_id   TEXT NOT NULL REFERENCES accounts(id),
    action_type  TEXT NOT NULL,                 -- post|reply|delete
    timestamp    TEXT NOT NULL DEFAULT (datetime('now')),
    count        INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_rate_account_action ON rate_tracking(account_id, action_type);
CREATE INDEX idx_rate_timestamp ON rate_tracking(timestamp);
```

---

## API Specification

### Base URL

`http://localhost:7845` (configurable via `TITEN_PORT`)

### Authentication

API key via header: `Authorization: Bearer <TITEN_API_KEY>`

### Response Format

All responses JSON. Errors:
```json
{ "error": "error message", "code": "ERROR_CODE" }
```

---

### Accounts

#### `GET /api/accounts`
List all accounts.
```json
// Response 200
{
  "data": [
    {
      "id": "019...",
      "username": "codecoradev",
      "user_id": "123456",
      "is_active": true,
      "expires_at": "2026-09-15T00:00:00Z",
      "token_status": "valid|expiring_soon|expired",
      "created_at": "..."
    }
  ]
}
```

#### `POST /api/accounts`
Register a new account.
```json
// Request
{
  "username": "codecoradev",
  "user_id": "123456",
  "access_token": "EAA...",
  "expires_at": "2026-09-15T00:00:00Z"
}

// Response 201
{ "data": { "id": "019...", "username": "codecoradev", ... } }
```

#### `PUT /api/accounts/:id`
Update account (token refresh, toggle active).
```json
// Request (token refresh)
{
  "access_token": "new_token...",
  "expires_at": "2026-11-15T00:00:00Z"
}
```

#### `DELETE /api/accounts/:id`
Remove account and all associated data.

#### `POST /api/accounts/:id/refresh-token`
Trigger manual token refresh via Threads API.

---

### Posts

#### `GET /api/posts?account_id=...&status=...&limit=...&offset=...`
List posts with optional filters.

#### `POST /api/posts`
Create and publish a post immediately.
```json
// Request
{
  "account_id": "019...",
  "media_type": "TEXT",
  "caption": "Hello Threads!",
  "text_attachment": {
    "plaintext": "Long form content here..."
  }
}

// Response 201
{ "data": { "id": "019...", "threads_post_id": "...", "status": "published" } }
```

#### `POST /api/posts` (media)
```json
{
  "account_id": "019...",
  "media_type": "IMAGE",
  "caption": "Photo caption",
  "image_url": "https://s3.../image.jpg",
  "alt_text": "Description"
}
```

#### `POST /api/posts` (carousel)
```json
{
  "account_id": "019...",
  "media_type": "CAROUSEL",
  "caption": "Carousel caption",
  "image_urls": ["https://...img1.jpg", "https://...img2.jpg"]
}
```

#### `GET /api/posts/:id`
Get post detail with cached insights.

#### `GET /api/posts/:id/insights`
Fetch fresh insights from Threads API (likes, replies, reposts, views, quotes).

#### `DELETE /api/posts/:id`
Delete post on Threads (requires `threads_delete` permission).

---

### Schedules

#### `GET /api/schedules?account_id=...&status=...`
List scheduled posts.

#### `POST /api/schedules`
Create a scheduled post (stored in SQLite, published at `scheduled_at`).
```json
{
  "account_id": "019...",
  "media_type": "TEXT",
  "caption": "Scheduled post",
  "scheduled_at": "2026-08-01T10:00:00+07:00"
}
```

#### `PUT /api/schedules/:id`
Update scheduled post (before it fires).

#### `DELETE /api/schedules/:id`
Cancel scheduled post.

#### `GET /api/schedules/upcoming`
List next N scheduled posts across all accounts (sorted by time).

---

### Comments

#### `GET /api/posts/:id/comments`
List fetched comments for a post (from local DB).

#### `POST /api/posts/:id/comments/fetch`
Trigger comment fetching from Threads API. Fetches all replies to the post, stores in DB.

#### `GET /api/posts/:id/comments/sentiment`
Get sentiment summary for a post's comments.
```json
{
  "data": {
    "total": 42,
    "positive": 28,
    "negative": 5,
    "neutral": 9,
    "average_score": 0.65,
    "comments": [
      { "text": "Great post!", "sentiment": "positive", "score": 0.92 },
      ...
    ]
  }
}
```

#### `POST /api/comments/:id/sentiment`
Run sentiment analysis on a single comment.

---

### Analytics

#### `GET /api/analytics/posts?account_id=...&from=...&to=...`
Get aggregated analytics across posts for an account (time range filter).
```json
{
  "data": {
    "total_posts": 15,
    "total_likes": 1200,
    "total_replies": 340,
    "total_reposts": 89,
    "total_views": 15000,
    "period": { "from": "...", "to": "..." },
    "posts": [
      { "post_id": "...", "caption": "...", "likes": 100, "replies": 30, ... }
    ]
  }
}
```

#### `GET /api/analytics/posts/:id/trend`
Get time-series analytics for a specific post (all snapshots).

#### `GET /api/analytics/sentiment/summary?account_id=...&from=...&to=...`
Overall sentiment summary across all posts in range.

---

### Media (S3)

#### `GET /api/media`
List uploaded media assets.

#### `POST /api/media/upload`
Upload file to S3. Multipart form upload.
```
Content-Type: multipart/form-data
file: <binary>
filename: image.jpg
content_type: image/jpeg
```
Response: `{ "data": { "id": "...", "s3_url": "https://...", "s3_key": "..." } }`

#### `GET /api/media/:id/url`
Get presigned URL for a media asset.

#### `DELETE /api/media/:id`
Delete media from S3 and DB.

---

### Proxy (Threads API passthrough)

#### `POST /api/proxy/:account_id/threads`
Proxy container creation to Threads API.
Same payload format as Threads Graph API, but routed through titen's rate limiter.

#### `POST /api/proxy/:account_id/threads/publish`
Proxy publish to Threads API.

#### `GET /api/proxy/:account_id/me/threads`
Proxy fetch own threads.

All proxy routes:
- Validate account + token
- Check rate limits before forwarding
- Log request to rate_tracking
- Return Threads API response transparently

---

### System

#### `GET /health`
Health check.
```json
{ "status": "ok", "version": "0.1.0", "db": "ok" }
```

---

## CLI Specification

Binary: `titen`

```
titen
├── serve                    # Start HTTP server
│   ├── --port <PORT>        # Default: 7845
│   └── --mcp                # Also enable MCP on stdio (Docker mode)
├── account
│   ├── list                 # List all accounts
│   ├── add <username> <user_id> <token> <expires_at>
│   ├── remove <id|username> # Remove account
│   ├── refresh <id|username># Refresh token
│   └── status <id|username> # Show token status + expiry
├── post
│   ├── create <account> --text "..." [--attachment "..."] [--type TEXT|IMAGE|VIDEO|CAROUSEL]
│   ├── delete <post_id>    # Delete on Threads
│   └── insights <post_id>   # Fetch + show insights
├── schedule
│   ├── list [--account <id>] [--status pending|published|failed]
│   ├── add <account> --text "..." --at "2026-08-01T10:00:00+07:00"
│   ├── cancel <id>          # Cancel scheduled post
│   └── upcoming             # Show next 10 upcoming
├── comment
│   ├── fetch <post_id>     # Fetch comments from Threads API
│   ├── list <post_id>      # Show stored comments
│   └── sentiment <post_id> # Analyze sentiment of post's comments
├── analytics
│   ├── posts <account> [--from ...] [--to ...]
│   ├── trend <post_id>     # Show time-series for a post
│   └── sentiment-summary <account>
├── media
│   ├── list
│   ├── upload <file_path> [--content-type image/jpeg]
│   └── delete <id>
└── token-check              # Check all accounts' token expiry status
```

### Example Usage

```bash
# Start server
titen serve --port 7845

# Add account
titen account add codecoradev 123456 "EAA..." "2026-09-15T00:00:00Z"

# Post immediately
titen post create codecoradev --text "Hello from titen!"

# Schedule post
titen schedule add codecoradev --text "Scheduled post" --at "2026-08-01T10:00:00+07:00"

# Fetch comments + analyze
titen comment fetch <post_id>
titen comment sentiment <post_id>

# Check token health
titen token-check
```

---

## MCP Server Specification

Runs on stdio as JSON-RPC (MCP protocol). Enabled via `titen serve --mcp` or separate binary `titen-mcp`.

### Tools

| Tool | Description | Parameters |
|------|-------------|------------|
| `list_accounts` | List all accounts | `active_only?: bool` |
| `get_account` | Get account details | `username: string` |
| `create_post` | Create and publish post | `account: string, text: string, media_type?: string, image_url?: string` |
| `schedule_post` | Schedule a post | `account: string, text: string, scheduled_at: string` |
| `list_schedules` | List scheduled posts | `account?: string, status?: string` |
| `cancel_schedule` | Cancel a scheduled post | `id: string` |
| `fetch_comments` | Fetch comments from Threads | `post_id: string` |
| `get_post_sentiment` | Get sentiment analysis | `post_id: string` |
| `get_post_analytics` | Get post analytics | `post_id: string` |
| `get_account_analytics` | Get account-level analytics | `account: string, from?: string, to?: string` |
| `upload_media` | Upload file to S3 | `file_path: string, content_type?: string` |
| `refresh_token` | Refresh account token | `username: string` |
| `check_tokens` | Check all token expiry | (none) |

### MCP JSON-RPC Example

```json
// Request
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "create_post",
    "arguments": {
      "account": "codecoradev",
      "text": "Hello from MCP!",
      "media_type": "TEXT"
    }
  }
}

// Response
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [{ "type": "text", "text": "{\"id\":\"019...\",\"threads_post_id\":\"...\",\"status\":\"published\"}" }]
  }
}
```

---

## Threads API Proxy

### How It Works

Instead of calling `graph.threads.net` directly, clients call titen proxy endpoints. Titen:

1. **Validates** the account token is active and not expired
2. **Checks rate limits** (sliding window from `rate_tracking` table)
3. **Forwards** the request to `graph.threads.net` with the account's token
4. **Logs** the action to `rate_tracking`
5. **Returns** the Threads API response transparently

### Rate Limiting Logic

```rust
// Per account, per action_type, per 24h sliding window
const LIMITS: &[(&str, i64)] = &[
    ("post",   250),
    ("reply",  1000),
    ("delete", 100),
];

fn check_rate_limit(account_id: &str, action: &str) -> Result<(), RateLimitError> {
    let window = now() - Duration::from_hours(24);
    let count = db.count_rate_actions(account_id, action, window);
    let limit = LIMITS.iter().find(|(a, _)| *a == action).map(|(_, l)| *l).unwrap_or(999);
    if count >= limit {
        return Err(RateLimitError { action, limit, remaining: 0 });
    }
    Ok(())
}
```

### Rate Limit Headers

Proxy responses include:
```
X-RateLimit-Limit: 250
X-RateLimit-Remaining: 243
X-RateLimit-Reset: 1692000000
```

---

## Scheduler

### Implementation

Uses `tokio-cron-scheduler` for recurring checks. The scheduler runs a tick every 60 seconds:

```
Every 60s:
  SELECT * FROM schedules
  WHERE status = 'pending'
    AND scheduled_at <= now()
  ORDER BY scheduled_at ASC

For each due schedule:
  1. Mark status = 'processing'
  2. Create Threads container (text or media)
  3. Wait appropriate delay (0s text, 30s image, 60s video)
  4. Publish container
  5. Create post record in `posts` table
  6. Update schedule: status = 'published', published_at = now(), result_json = response
  7. On error: status = 'failed', error = message (allow retry)
```

### Retry Logic

- Failed schedules can be retried manually via `POST /api/schedules/:id/retry`
- No automatic retry (to avoid infinite loops on permanent errors)
- User can inspect error and fix (e.g., expired media URL, token issue)

---

## S3 / Media Management

### Configuration

All S3 config via ENV:

| ENV | Description | Example |
|-----|-------------|---------|
| `TITEN_S3_ENDPOINT` | S3 endpoint URL | `https://minio.example.com` |
| `TITEN_S3_REGION` | S3 region | `us-east-1` |
| `TITEN_S3_BUCKET` | Bucket name | `titen-media` |
| `TITEN_S3_ACCESS_KEY` | Access key ID | |
| `TITEN_S3_SECRET_KEY` | Secret access key | |
| `TITEN_S3_PUBLIC_URL` | Base URL for public access | `https://media.example.com/titen-media` |

### Upload Flow

```
1. Client uploads file via POST /api/media/upload
2. Titen saves to S3 at: {bucket}/{YYYY}/{MM}/{DD}/{uuid}.{ext}
3. Titen stores record in media_assets table
4. Returns s3_url (public URL if configured, or presigned)
5. Client uses s3_url as image_url/video_url in post creation
```

### Storage Trait

```rust
#[async_trait]
pub trait Storage: Send + Sync {
    async fn upload(&self, key: &str, data: &[u8], content_type: &str) -> Result<String>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn get_url(&self, key: &str) -> Result<String>;
    async fn list(&self, prefix: &str) -> Result<Vec<StorageEntry>>;
}
```

Default implementation: `rust-s3` client. All operations go through the trait, so it's swappable for testing (in-memory) or alternative backends.

---

## Sentiment Analysis

### Trait Design

```rust
pub struct SentimentResult {
    pub label: String,      // "positive" | "negative" | "neutral"
    pub score: f64,         // -1.0 to 1.0
}

#[async_trait]
pub trait SentimentEngine: Send + Sync {
    async fn analyze(&self, text: &str) -> Result<SentimentResult>;
    async fn analyze_batch(&self, texts: &[&str]) -> Result<Vec<SentimentResult>>;
}
```

### Implementations

| Engine | Status | Description |
|--------|--------|-------------|
| **StubEngine** | MVP | Always returns `neutral, 0.0`. For development/testing. |
| **OnnxEngine** | Future | Local ONNX model (Indonesian + English). No external API. |
| **LlmEngine** | Future | Proxy to configurable LLM API (OpenAI-compatible). ENV-driven. |
| **CustomApiEngine** | Future | Generic HTTP POST to user-defined sentiment API endpoint. |

### Selection via ENV

```
TITEN_SENTIMENT_ENGINE=stub       # default
TITEN_SENTIMENT_ENGINE=onnx
TITEN_SENTIMENT_ENGINE=llm
TITEN_SENTIMENT_ENGINE=custom_api
TITEN_SENTIMENT_API_URL=https://...
TITEN_SENTIMENT_API_KEY=...
```

### Batch Analysis Flow

```
1. fetch_comments (store raw in comments table, sentiment = null)
2. POST /api/posts/:id/comments/sentiment
3. Load all comments where sentiment IS NULL
4. Call engine.analyze_batch(comments)
5. Update comments table with results
6. Return aggregate summary
```

---

## Token Lifecycle Management

### Token States

```
valid ──────────> expiring_soon (7 days before expiry)
  │                        │
  │ (auto-refresh)         │ (auto-refresh if within 7 days)
  ▼                        ▼
valid <────────── refreshed
  │
  └──> expired (past expiry, refresh failed)
        │
        └──> needs_reauth (manual OAuth required)
```

### Auto-Refresh Logic

The scheduler checks token expiry every 6 hours:

```
For each active account:
  if expires_at - now < 7 days AND expires_at > now:
    refresh_token(account)
  if expires_at <= now:
    mark account as needs_reauth
    log warning
```

### Refresh Endpoint

```
GET graph.threads.net/refresh_access_token
  ?grant_type=th_refresh_token
  &access_token={current_token}
```

Returns new token + new `expires_in` (~60 days).

### Rate Tracking on Token Refresh

Token refreshes are also tracked to prevent abuse (Meta may rate-limit refresh attempts).

---

## Configuration

All configuration via environment variables. No config files.

| ENV | Default | Description |
|-----|---------|-------------|
| `TITEN_PORT` | `7845` | HTTP server port |
| `TITEN_API_KEY` | (none) | API key for HTTP auth. If unset, no auth required (dev mode). |
| `TITEN_DB_PATH` | `./titen.db` | SQLite database path |
| `TITEN_LOG_LEVEL` | `info` | Log level (trace, debug, info, warn, error) |
| `TITEN_HOST` | `0.0.0.0` | Bind address |
| `TITEN_THREADS_APP_ID` | (none) | Meta App ID (for token refresh) |
| `TITEN_THREADS_APP_SECRET` | (none) | Meta App Secret (for token exchange only) |
| **S3** | | |
| `TITEN_S3_ENDPOINT` | (none) | S3 endpoint URL |
| `TITEN_S3_REGION` | `us-east-1` | S3 region |
| `TITEN_S3_BUCKET` | `titen-media` | Bucket name |
| `TITEN_S3_ACCESS_KEY` | (none) | Access key |
| `TITEN_S3_SECRET_KEY` | (none) | Secret key |
| `TITEN_S3_PUBLIC_URL` | (none) | Base URL for public access |
| **Sentiment** | | |
| `TITEN_SENTIMENT_ENGINE` | `stub` | Engine type: stub, onnx, llm, custom_api |
| `TITEN_SENTIMENT_API_URL` | (none) | Custom API URL |
| `TITEN_SENTIMENT_API_KEY` | (none) | Custom API key |
| **Scheduler** | | |
| `TITEN_SCHEDULER_INTERVAL_SECS` | `60` | How often to check for due schedules |
| `TITEN_TOKEN_CHECK_INTERVAL_HOURS` | `6` | How often to check token expiry |

### ENV Validation

On startup, titen validates required ENV vars and exits with clear error messages if missing:

```
ERROR: TITEN_DB_PATH is required
ERROR: TITEN_S3_ENDPOINT is required when using media features
```

Optional features (S3, sentiment) only validated when the feature is actually used.

---

## Docker Deployment

### Dockerfile (cargo-chef)

```dockerfile
# Stage 1: chef
FROM rust:1.88-slim-bookworm AS chef
RUN cargo install cargo-chef
WORKDIR /app

# Stage 2: planner
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: builder
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --workspace

# Stage 4: runtime
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/titen /usr/local/bin/
COPY --from=builder /app/target/release/titen-api /usr/local/bin/
EXPOSE 7845
CMD ["titen", "serve", "--port", "7845"]
```

### Docker Compose (example)

```yaml
services:
  titen:
    build: .
    ports:
      - "7845:7845"
    environment:
      TITEN_DB_PATH: /data/titen.db
      TITEN_API_KEY: ${TITEN_API_KEY}
      TITEN_THREADS_APP_ID: ${TITEN_THREADS_APP_ID}
      TITEN_S3_ENDPOINT: http://minio:9000
      TITEN_S3_REGION: us-east-1
      TITEN_S3_BUCKET: titen-media
      TITEN_S3_ACCESS_KEY: ${MINIO_ROOT_USER}
      TITEN_S3_SECRET_KEY: ${MINIO_ROOT_PASSWORD}
      TITEN_S3_PUBLIC_URL: https://media.example.com/titen-media
      TITEN_LOG_LEVEL: info
    volumes:
      - titen-data:/data
    depends_on:
      - minio

  minio:
    image: minio/minio:latest
    ports:
      - "9000:9000"
      - "9001:9001"
    environment:
      MINIO_ROOT_USER: ${MINIO_ROOT_USER}
      MINIO_ROOT_PASSWORD: ${MINIO_ROOT_PASSWORD}
    command: server /data --console-address ":9001"
    volumes:
      - minio-data:/data

volumes:
  titen-data:
  minio-data:
```

### Traefik Integration

```yaml
# docker-compose.traefik.yml addition
services:
  titen:
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.titen.rule=Host(`titen.ajianaz.dev`)"
      - "traefik.http.routers.titen.tls.certresolver=letsencrypt"
      - "traefik.http.services.titen.loadbalancer.server.port=7845"
```

---

## Development Roadmap

### Phase 1: MVP (Core + CLI)
- [ ] Cargo workspace scaffolding
- [ ] SQLite schema + migrations
- [ ] Account management (CRUD + token tracking)
- [ ] Token refresh (manual + scheduled)
- [ ] Post creation (text only, one-step)
- [ ] Schedule (create, list, cancel)
- [ ] Scheduler (tokio-cron, 60s tick)
- [ ] CLI (account, post, schedule subcommands)
- [ ] Basic rate limit tracking

### Phase 2: HTTP API + Proxy
- [ ] Axum HTTP server
- [ ] All CRUD endpoints
- [ ] Threads API proxy with rate limiting
- [ ] Media post (IMAGE, two-step)
- [ ] Carousel post
- [ ] API key auth middleware
- [ ] Dockerfile + Docker Compose
- [ ] Integration tests

### Phase 3: Comments + Analytics
- [ ] Comment fetching from Threads API
- [ ] Comment storage in SQLite
- [ ] Post insights fetching + time-series storage
- [ ] Analytics aggregation endpoints
- [ ] Sentiment stub engine
- [ ] Sentiment analysis endpoints

### Phase 4: S3 + MCP
- [ ] S3 storage trait + rust-s3 impl
- [ ] Media upload/download/delete endpoints
- [ ] MCP server (stdio JSON-RPC)
- [ ] MCP tools (all major operations)
- [ ] `titen serve --mcp` mode

### Phase 5: Advanced
- [ ] ONNX sentiment engine
- [ ] LLM/custom API sentiment engine
- [ ] Token auto-refresh (7-day window)
- [ ] Bulk comment sentiment analysis
- [ ] Analytics trend visualization data export
- [ ] Traefik integration guide

---

## Naming Conventions

Follows `codecoradev` monorepo conventions:

- **Directory names**: hyphens (`titen-core/`, `titen-api/`)
- **Package names**: underscores (`titen_core`, `titen_api`)
- **Binary names**: hyphens (`titen`, `titen-api`, `titen-cli`, `titen-mcp`)
- **CLI commands**: single words or hyphenated (`account list`, `token-check`)
- **API routes**: kebab-case (`/api/schedules/upcoming`)
- **DB columns**: snake_case (`access_token`, `scheduled_at`)
- **ENV vars**: `TITEN_` prefix, snake_case (`TITEN_DB_PATH`, `TITEN_S3_ENDPOINT`)
- **Error codes**: SCREAMING_SNAKE_CASE (`RATE_LIMIT_EXCEEDED`, `TOKEN_EXPIRED`)

### Binary Entry Points

| Binary | Crate | Purpose |
|--------|-------|---------|
| `titen` | `titen-cli` | CLI interface (default binary) |
| `titen-api` | `titen-api` | HTTP server |
| `titen-mcp` | `titen-mcp` | MCP server (stdio) |

Single `titen` binary with subcommands is the primary interface. `titen-api` and `titen-mcp` are separate binaries for Docker/container deployment where you want a single-purpose process.
