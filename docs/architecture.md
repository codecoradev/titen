# Architecture Overview

Titen is a self-hosted Threads management platform built as a Rust workspace (4 crates) with a SvelteKit frontend. This document covers the system layout, crate structure, database schema, and key runtime flows.

---

## System Diagram

```
┌─────────────────────────────────────────────────────────┐
│                      Browser Client                      │
│                   SvelteKit Frontend                     │
└──────────────┬──────────────────────────┬───────────────┘
               │  /api/* (REST + cookies)  │  /auth/* (OAuth)
               ▼                           ▼
┌─────────────────────────────────────────────────────────┐
│                    Axum HTTP Server                      │
│                      (titen-api)                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────────┐  │
│  │API Key   │  │  CORS    │  │   Rate Limiting      │  │
│  │Auth MW   │  │  Layer   │  │   (rate_tracking)     │  │
│  └──────────┘  └──────────┘  └──────────────────────┘  │
└──────┬────────────────┬─────────────────┬──────────────┘
       │                │                  │
       ▼                ▼                  ▼
┌─────────────┐  ┌──────────────┐  ┌────────────────┐
│  SQLite DB  │  │   Threads    │  │  S3 Storage    │
│ (7 tables)  │  │  Graph API   │  │ (media assets) │
│             │  │              │  │                │
│ accounts    │  │ - Publish    │  │ - Upload       │
│ posts       │  │ - Replies    │  │ - Retrieve     │
│ schedules   │  │ - Analytics  │  │                │
│ comments    │  │ - Media      │  │                │
│ analytics_  │  │              │  │                │
│   snap      │  │              │  │                │
│ media_assets│  │              │  │                │
│ rate_       │  │              │  │                │
│   tracking  │  │              │  │                │
└─────────────┘  └──────────────┘  └────────────────┘
```

---

## Crate Dependency Graph

```
    ┌───────────┐
    │ titen-cli │ (Clap CLI)
    └─────┬─────┘
          │ depends on
          ▼
    ┌───────────┐
    │ titen-api │ (Axum HTTP server)
    └─────┬─────┘
          │ depends on
          ▼
    ┌─────────────┐         ┌───────────┐
    │ titen-core  │◄────────│ titen-mcp │ (MCP stdio server)
    └─────────────┘         └───────────┘
```

Both `titen-api` and `titen-mcp` depend on `titen-core` for domain logic.
`titen-cli` depends on `titen-api` (and transitively `titen-core`) to launch
the full server.

---

## Crates

### titen-core

The foundation crate. Contains all domain logic and infrastructure integration.

| Responsibility | Details |
|---|---|
| **Models** | Domain types: `Account`, `Post`, `Schedule`, `Comment`, `AnalyticsSnapshot`, `MediaAsset`, `RateTracking` |
| **SQLite store** | Connection pooling, queries, and persistence via `rusqlite` / `sqlx` |
| **Threads client** | Graph API wrapper — publish, reply, fetch analytics, upload media |
| **Scheduler** | Cron-driven job engine that processes due schedules |
| **Sentiment trait** | Extensible sentiment analysis trait (pluggable backends) |
| **S3 storage** | Object storage abstraction for media assets |

### titen-api

The HTTP server crate, built on **Axum**.

- **REST API** — all endpoints under `/api/*`
- **API key auth** — `api_key_auth` middleware layer
- **CORS** — configurable via `TITEN_CORS_ORIGINS` environment variable. Only explicitly listed origins are allowed; malformed entries are silently skipped. No catch-all origin pattern is permitted. Default: same-origin only.
- **Rate limiting** — backed by the `rate_tracking` table in SQLite

### titen-cli

The command-line entry point, built with **Clap**.

- Parses CLI arguments (port, config path, environment)
- Initializes logging
- Starts the Axum server via `titen-api`
- Provides admin subcommands (migration runner, diagnostics)

### titen-mcp

An **MCP (Model Context Protocol) stdio server** for AI agent integration.

- Exposes **14 tools** covering accounts, posts, schedules, analytics, and media
- Communicates over stdin/stdout using the MCP protocol
- Delegates all logic to `titen-core`

---

## Database Schema

SQLite is the sole database. There are **7 tables** across **3 migrations**.

### Tables

| # | Table | Purpose |
|---|---|---|
| 1 | `accounts` | Connected Threads accounts (OAuth tokens, metadata) |
| 2 | `posts` | Draft and published post content |
| 3 | `schedules` | Scheduled post entries (status, scheduled_at, claimed_at) |
| 4 | `comments` | Comments and replies fetched from Threads |
| 5 | `analytics_snap` | Analytics snapshots (views, likes, replies, reposts) |
| 6 | `media_assets` | Media metadata and S3 references |
| 7 | `rate_tracking` | API rate limit counters per endpoint/window |

### Migrations

| Migration | Name | Description |
|---|---|---|
| 1 | `001_initial` | Creates all 7 base tables, indexes, and initial schema |
| 2 | `002_drop_refresh_token` | Removes the `refresh_token` column from `accounts` (Threads uses long-lived access tokens) |
| 3 | `003_add_app_secret` | Adds `app_secret` column to `accounts` for per-account signing |

---

## Request Flow

```
SvelteKit FE
    │
    │  HTTP request (cookie or X-API-Key header)
    ▼
/api/*
    │
    ▼
api_key_auth middleware ──► validates TITEN_API_KEY
    │                       (or session cookie)
    │ pass
    ▼
Route Handler (Axum)
    │
    ▼
Store (titen-core)
    │
    ▼
SQLite DB
```

1. The SvelteKit frontend sends a request to `/api/*` with either an `X-API-Key` header or the `titen_session` cookie.
2. The `api_key_auth` middleware extracts and validates the credential.
3. On success, the Axum route handler processes the request using the `titen-core` store.
4. The store executes SQL against SQLite and returns results.

---

## Scheduler Flow

The scheduler is a **tokio-cron** task that runs inside the API server process.

```
tokio-cron tick (every N seconds)
    │
    ▼
process_due_schedules()
    │  SELECT schedules WHERE status='pending' AND scheduled_at <= now()
    ▼
claim_schedule() ──► atomic UPDATE ... WHERE status='pending' (row lock)
    │                 prevents double-posting in HA setups
    │ claimed
    ▼
Threads client ──► publish to Threads Graph API
    │
    ▼
update schedule status ──► 'published' or 'failed'
    │
    ▼
store analytics / media references in SQLite
```

Key design points:

- **Atomic claim** — `claim_schedule()` performs an `UPDATE ... SET status='claimed' WHERE id=? AND status='pending'` so multiple server instances cannot double-post.
- **Failure handling** — failed publishes are marked `'failed'` and can be retried.
- **Token usage** — the scheduler reads the connected account's access token from the `accounts` table to authenticate with the Threads API.

---

## Authentication Model

Titen uses a **two-tier** authentication model:

| Tier | Mechanism | Scope |
|---|---|---|
| **Server-level (admin)** | Single `TITEN_API_KEY` environment variable | Grants access to the REST API and web dashboard |
| **Per-account (Threads)** | OAuth 2.0 via Threads Graph API | Connects individual Threads accounts for posting |

- The `TITEN_API_KEY` is a shared secret configured at deployment. It is validated by the `api_key_auth` middleware on every `/api/*` request.
- For the **web dashboard**, an `httpOnly` cookie session (`titen_session`) is issued after login. The cookie has `SameSite=Strict` and a 7-day expiry (`Max-Age=604800`).
- **OAuth** is used exclusively for connecting Threads accounts — it does **not** authenticate admin users. The OAuth tokens stored in the `accounts` table are used by the scheduler to publish on behalf of each account.

> **Dev mode:** When `TITEN_API_KEY` is not set, all endpoints are open and no authentication is required. This is intended for local development only.
