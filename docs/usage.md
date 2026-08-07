# Usage Guide

This guide walks through everyday tasks with Titen, from connecting your first Threads account to scheduling posts and reading analytics.

> **New here?** Follow the [Deployment Guide](deployment.md) first to get the server running.

---

## Table of Contents

- [Web Dashboard](#web-dashboard)
- [Connecting a Threads Account](#connecting-a-threads-account)
- [Creating Posts](#creating-posts)
- [Scheduling Posts](#scheduling-posts)
- [Managing Comments](#managing-comments)
- [Viewing Analytics](#viewing-analytics)
- [Uploading Media](#uploading-media)
- [CLI Quick Reference](#cli-quick-reference)
- [MCP / AI Agent Integration](#mcp--ai-agent-integration)

---

## Web Dashboard

Titen includes a built-in web dashboard served at the root URL.

### Login

1. Navigate to `http://your-server:7845/login` (or your domain).
2. Enter your API key (the value of `TITEN_API_KEY`).
3. Click **Login**. A session cookie is set (valid for 7 days).

> **Dev mode:** If `TITEN_API_KEY` is not set, the dashboard is accessible without login. Never use this in production.

### Dashboard Sections

| Section | URL | Purpose |
|---------|-----|---------|
| **Accounts** | `/admin/accounts` | Manage connected Threads accounts |
| **Posts** | `/admin/posts` | View and create posts |
| **Schedules** | `/admin/schedules` | View and create scheduled posts |
| **Comments** | `/admin/comments` | Browse comment sentiment per post |
| **Analytics** | `/admin/analytics` | Performance metrics over time |

---

## Connecting a Threads Account

Titen uses the Threads Graph API. You need a Threads account and a Meta for Developers app.

### Step 1: Get API Credentials

1. Go to [Meta for Developers](https://developers.facebook.com/apps).
2. Create a new app (type: **Business**).
3. Add the **Threads API** product.
4. Navigate to **Threads → API Setup** to find your App ID and App Secret.
5. Generate a long-lived access token via the OAuth flow or the Token Generator tool.

### Step 2: Add Account via CLI

```bash
# Set your API key (if auth is enabled)
export TITEN_API_KEY=your-key
export TITEN_URL=http://localhost:7845

# Add the account
titen account add mybrand \
  --access-token "YOUR_LONG_LIVED_TOKEN" \
  --user-id "THREADS_USER_ID" \
  --expires-at "2026-12-01T00:00:00Z"
```

### Step 3: Add Account via Dashboard

Alternatively, use the web dashboard:

1. Go to **Accounts** → click **Add Account**.
2. Enter username, access token, and optional user ID.
3. Click **Save**.

### Step 4: Add Account via OAuth (recommended)

The cleanest method, no manual token copying:

1. Set these environment variables on the server:
   ```
   THREADS_APP_ID=your_app_id
   THREADS_APP_SECRET=your_app_secret
   THREADS_REDIRECT_URI=https://your-domain.com/auth/callback
   ```
2. From the dashboard **Accounts** page, click **Connect via OAuth**.
   Or call the API:
   ```bash
   curl -X POST http://localhost:7845/api/threads/oauth/initiate \
     -H "X-API-Key: your-key"
   ```
3. You'll be redirected to Threads to authorize. After approval, the token is stored automatically.

### Verify Connection

```bash
# List all accounts
titen account list

# Check token validity
titen token-check

# Fetch profile (confirms API connectivity)
titen account status <account_id>
```

---

## Creating Posts

### Post Types

| Type | Fields Required | Notes |
|------|-----------------|-------|
| `TEXT` | `text` | Up to 500 characters (Threads limit) |
| `IMAGE` | `image_url` or uploaded media | One image per post |
| `VIDEO` | `video_url` | Processed by Threads (may take up to 2 min) |

### Via CLI

```bash
# Text post (published immediately)
titen post create mybrand --text "Hello from Titen!"

# Image post
titen post create mybrand \
  --media-type IMAGE \
  --image-url "https://example.com/photo.jpg" \
  --text "Check this out"

# Video post
curl -X POST http://localhost:7845/api/posts \
  -H "X-API-Key: your-key" \
  -H "Content-Type: application/json" \
  -d '{
    "account_id": 1,
    "media_type": "VIDEO",
    "video_url": "https://example.com/video.mp4"
  }'
```

### Via API

```bash
# Text post
curl -X POST http://localhost:7845/api/posts \
  -H "X-API-Key: your-key" \
  -H "Content-Type: application/json" \
  -d '{
    "account_id": 1,
    "media_type": "TEXT",
    "text": "Hello from Titen!"
  }'

# Image post
curl -X POST http://localhost:7845/api/posts \
  -H "X-API-Key: your-key" \
  -H "Content-Type: application/json" \
  -d '{
    "account_id": 1,
    "media_type": "IMAGE",
    "image_url": "https://example.com/photo.jpg"
  }'

# Video post
curl -X POST http://localhost:7845/api/posts \
  -H "X-API-Key: your-key" \
  -H "Content-Type: application/json" \
  -d '{
    "account_id": 1,
    "media_type": "VIDEO",
    "video_url": "https://example.com/video.mp4"
  }'
```

### Via Dashboard

1. Go to **Posts** → click **New Post**.
2. Select account, choose media type, enter text or URL.
3. Click **Publish** for immediate posting.

---

## Scheduling Posts

The scheduler runs as a background task inside the server. It checks for due schedules every 60 seconds (configurable via `TITEN_SCHEDULER_INTERVAL_SECS`).

### Create a Schedule

```bash
# Schedule a text post for tomorrow at 9 AM
titen schedule add mybrand \
  --text "Good morning!" \
  --at "2026-08-05T09:00:00+07:00"

# Schedule an image post
titen schedule add mybrand \
  --text "Product launch!" \
  --media-type IMAGE \
  --at "2026-08-05T14:00:00+07:00"
```

### Via API

```bash
curl -X POST http://localhost:7845/api/schedules \
  -H "X-API-Key: your-key" \
  -H "Content-Type: application/json" \
  -d '{
    "account_id": 1,
    "media_type": "TEXT",
    "text": "Scheduled post content",
    "scheduled_at": "2026-08-05T09:00:00+07:00"
  }'
```

### Manage Schedules

```bash
# List all schedules
titen schedule list

# Filter by account or status
titen schedule list --account 1 --status pending

# View upcoming (next 10)
titen schedule upcoming

# Cancel a schedule
titen schedule cancel <schedule_id>
```

### How Scheduling Works

```
Schedule created (status: pending)
    │
    ▼
Scheduler tick (every 60s)
    │
    ├── claim_schedule() - atomic UPDATE WHERE status='pending'
    │   (prevents double-posting if running multiple instances)
    │
    ├── status → processing
    │
    ├── Create media container on Threads API
    │
    ├── Wait for container ready:
    │   • TEXT: immediate
    │   • IMAGE: ~30 seconds
    │   • VIDEO: up to 4.5 minutes (polled, timeout-bounded)
    │
    ├── Publish container
    │
    └── status → published (or failed on error)
```

---

## Managing Comments

### Fetch Comments from Threads

Comments are not synced automatically. Fetch them on demand:

```bash
# Fetch latest comments for a post
titen comment fetch <post_id>

# List stored comments
titen comment list <post_id>
```

### Sentiment Analysis

```bash
# Analyze sentiment of stored comments
titen comment sentiment <post_id>

# Summary across a post
titen analytics sentiment-summary <post_id>
```

The sentiment engine is pluggable:

| Engine | Env Var Value | Description |
|--------|-------------|-------------|
| Keyword (default) | `keyword` | Simple keyword-based scoring, no deps |
| ONNX | `onnx` | Local ML model (requires model file) |
| LLM | `llm` | Uses an LLM API (requires API key) |
| Custom API | `custom_api` | Your own sentiment endpoint |

Set via: `TITEN_SENTIMENT_ENGINE=keyword`

---

## Viewing Analytics

### Post-Level Analytics

```bash
# Fetch and store insights from Threads
titen post insights <post_id>

# Time-series trend for a post
titen analytics trend <post_id>
```

### Account-Level Analytics

```bash
# Summary for an account (date range)
titen analytics posts mybrand --from 2026-08-01 --to 2026-08-31
```

### Via API

```bash
# Post analytics
curl http://localhost:7845/api/analytics/posts?account_id=1\&from=2026-08-01\&to=2026-08-31 \
  -H "X-API-Key: your-key"

# Trend for a specific post
curl http://localhost:7845/api/analytics/posts/42/trend \
  -H "X-API-Key: your-key"
```

---

## Uploading Media

Media uploads require S3-compatible storage to be configured.

### Configure S3

Set these environment variables:

```bash
TITEN_S3_ENDPOINT=https://s3.example.com
TITEN_S3_BUCKET=titen-media
TITEN_S3_REGION=us-east-1
TITEN_S3_ACCESS_KEY=your-access-key
TITEN_S3_SECRET_KEY=your-secret-key
TITEN_S3_PUBLIC_URL=https://cdn.example.com  # optional, for public access
```

> **MinIO** works great for self-hosted setups. See the [Deployment Guide](deployment.md#s3-media-storage-optional).

### Upload via CLI

```bash
titen media upload /path/to/image.jpg --content-type image/jpeg
titen media list
titen media delete <media_id>
```

### Upload via API

```bash
curl -X POST http://localhost:7845/api/media \
  -H "X-API-Key: your-key" \
  -F "file=@/path/to/image.jpg"
```

---

## CLI Quick Reference

```
titen serve [--host 0.0.0.0] [--port 7845] [--mcp]

# Accounts
titen account list
titen account add <username> --access-token <TOKEN> [--user-id <ID>] [--expires-at <ISO>]
titen account remove <id>
titen account refresh <id>
titen account status <id>
titen token-check

# Posts
titen post create <account> --text <TEXT> [--media-type TEXT|IMAGE] [--image-url <URL>]
titen post delete <post_id>
titen post insights <post_id>

# Schedules
titen schedule add <account> --text <TEXT> --at <ISO8601> [--media-type TEXT|IMAGE]
titen schedule list [--account <id>] [--status <status>]
titen schedule cancel <id>
titen schedule upcoming

# Comments
titen comment fetch <post_id>
titen comment list <post_id>
titen comment sentiment <post_id>

# Analytics
titen analytics posts <account> [--from <date>] [--to <date>]
titen analytics trend <post_id>
titen analytics sentiment-summary <post_id>

# Media
titen media list
titen media upload <file_path> [--content-type <mime>]
titen media delete <id>
```

---

## MCP / AI Agent Integration

Titen ships with an MCP server (`titen-mcp`) for integration with AI tools.

### Claude Desktop

Add to `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS) or `~/.config/Claude/claude_desktop_config.json` (Linux):

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

### Cursor

Add to your MCP settings (`Settings → MCP`):

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

### Available MCP Tools (14)

| Tool | Description |
|------|-------------|
| `list_accounts` | List all Threads accounts |
| `get_account` | Get account by ID |
| `create_post` | Create and publish a post |
| `schedule_post` | Schedule a post |
| `list_schedules` | List scheduled posts |
| `cancel_schedule` | Cancel a schedule |
| `fetch_comments` | Fetch comments from Threads |
| `get_post_sentiment` | Analyze comment sentiment |
| `get_post_analytics` | Analytics for a post |
| `get_account_analytics` | Analytics summary per account |
| `upload_media` | Upload media to S3 |
| `refresh_token` | Refresh an account token |
| `check_tokens` | Batch token expiry check |

### Example: Ask Claude to Post

> "Use titen to post 'Hello world from my AI assistant!' to my account @mybrand."

Claude will call `create_post` with the appropriate parameters.
