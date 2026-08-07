# Titen Deployment Guide

This guide covers everything you need to deploy Titen in a production environment.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Prerequisites](#2-prerequisites)
3. [Quick Deploy (Docker, recommended)](#3-quick-deploy-docker--recommended)
4. [Native Deploy (binary)](#4-native-deploy-binary)
5. [Reverse Proxy Configuration](#5-reverse-proxy-configuration)
6. [Threads API Setup](#6-threads-api-setup)
7. [S3 Media Storage (optional)](#7-s3-media-storage-optional)
8. [Security Checklist](#8-security-checklist)
9. [Backup & Recovery](#9-backup--recovery)
10. [Updating](#10-updating)
11. [Troubleshooting](#11-troubleshooting)

---

## 1. Overview

This guide covers production deployment options for **Titen**, a self-hosted Threads management platform.

**What this guide covers:**

- Docker-based deployment (recommended)
- Native binary deployment with systemd
- Reverse proxy configuration (Caddy / Nginx) for TLS termination
- Threads API OAuth setup
- Optional S3-compatible media storage
- Security hardening, backup, recovery, updating, and troubleshooting

### Architecture Reminder

Titen is built as a **single self-contained binary** with three core components:

| Component | Description |
|---|---|
| **Rust binary** | HTTP server (Axum), scheduler, CLI, MCP server, all in one |
| **Static web assets** | Pre-built SvelteKit dashboard served from `/app/web` inside Docker |
| **SQLite database** | Single-file database at `/data/titen.db` (Docker) or `~/.codecora/titen/titen.db` (local) |

No external database, Redis, or message queue is required. The binary is fully self-contained.

> **Default port:** `7845`. The binary listens on this port for HTTP traffic. In production, place it behind a reverse proxy.

---

## 2. Prerequisites

### Threads API Credentials

Titen integrates with the **Meta Threads API**. You need a Meta for Developers app with Threads API access.

**How to get credentials:**

1. Go to [Meta for Developers](https://developers.facebook.com/)
2. Create a new app (or use an existing one)
3. Navigate to **Add Product → Threads API**
4. Configure the OAuth redirect URI to point to your Titen instance:
   ```
   https://your-domain.com/auth/callback
   ```
5. Note your **App ID** and **App Secret**
6. Generate an initial **access token** via the Threads API settings

> Detailed step-by-step in [Section 6, Threads API Setup](#6-threads-api-setup).

### Runtime Requirements

| Requirement | Docker Deploy | Native Deploy |
|---|---|---|
| **Docker** | Docker 24+ with Docker Compose v2 | Not required |
| **Rust toolchain** | Not required | Rust 1.85+ (only if building from source) |
| **OS** | Any OS that runs Docker | Linux (x86_64 or aarch64) |
| **RAM** | 512 MB minimum | 256 MB minimum |
| **Disk** | 1 GB minimum (plus SQLite + media) | 50 MB binary + data |

### Reverse Proxy

A **reverse proxy** is required in production for:

- **TLS termination** (HTTPS)
- **Domain routing**
- **Request size limits** (for media uploads)
- **Security** (Titen should not be exposed directly on port 7845)

Supported: **Caddy** (recommended for automatic HTTPS) or **Nginx**.

---

## 3. Quick Deploy (Docker, recommended)

Docker is the fastest and most reliable way to deploy Titen.

### Step 1: Clone the Repository

```bash
git clone https://github.com/codecoradev/titen.git
cd titen
```

### Step 2: Create a Production docker-compose.yml

Create `docker-compose.yml` in your deployment directory:

```yaml
version: "3.9"

services:
  titen:
    image: codecoradev/titen:latest
    container_name: titen
    restart: unless-stopped
    ports:
      - "127.0.0.1:7845:7845"  # Bind to localhost only; reverse proxy handles external traffic
    volumes:
      - titen-data:/data       # Persistent SQLite database + config
    environment:
      # ── Core ──
      TITEN_DB_PATH: "/data/titen.db"
      TITEN_API_KEY: "your-secure-api-key-here"       # MANDATORY in production
      TITEN_ENCRYPTION_KEY: "your-encryption-key-here" # AES-256-GCM key for token encryption. Generate: openssl rand -hex 32
      TITEN_REQUIRE_ENCRYPTION: "true"                 # Fail-fast if encryption key is missing
      TITEN_HOST: "0.0.0.0"
      TITEN_PORT: "7845"
      TITEN_URL: "https://titen.yourdomain.com"       # Public-facing URL
      TITEN_WEB_DIR: "/app/web"                        # Static assets path in container

      # ── CORS ──
      TITEN_CORS_ORIGINS: "https://titen.yourdomain.com"

      # ── Scheduler ──
      TITEN_SCHEDULER_INTERVAL_SECS: "60"

      # ── Sentiment Engine ──
      TITEN_SENTIMENT_ENGINE: "rule-based"             # or "ai" if configured

      # ── S3 Media Storage (optional, uncomment if using) ──
      # TITEN_S3_ENDPOINT: "https://s3.amazonaws.com"
      # TITEN_S3_BUCKET: "titen-media"
      # TITEN_S3_REGION: "us-east-1"
      # TITEN_S3_ACCESS_KEY: "your-access-key"
      # TITEN_S3_SECRET_KEY: "your-secret-key"
      # TITEN_S3_PUBLIC_URL: "https://titen-media.s3.amazonaws.com"

    healthcheck:
      test: ["CMD", "wget", "--spider", "-q", "http://localhost:7845/api/health"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s

volumes:
  titen-data:
```

### Step 3: Set Your API Key

Generate a secure API key and set it as `TITEN_API_KEY`:

```bash
# Generate a random key
openssl rand -hex 32
```

Set this value in your `docker-compose.yml` or via a `.env` file:

```bash
# .env file (same directory as docker-compose.yml)
TITEN_API_KEY=your-generated-key-here
```

### Step 4: Start the Container

```bash
docker compose up -d
```

### Step 5: Verify Health

```bash
# Check container status
docker compose ps

# Check health endpoint
curl http://localhost:7845/api/health

# View logs
docker logs titen
```

You should see a `200 OK` response from the health endpoint and startup logs showing successful database initialization.

### Volume Persistence

The `titen-data` volume is mounted at `/data` inside the container and stores:

- **SQLite database** (`/data/titen.db`): all posts, schedules, accounts, analytics
- **Uploaded media** (if stored locally rather than S3)
- **Configuration state**

This volume **survives container restarts and updates**. Without it, all data would be lost when the container is recreated.

To inspect the volume:

```bash
docker volume inspect titen_titen-data
```

---

## 4. Native Deploy (binary)

If you prefer not to use Docker, you can run Titen as a native binary managed by systemd.

### Option A: Download Pre-built Binary

Download the latest release for your architecture from the [GitHub Releases page](https://github.com/codecoradev/titen/releases):

```bash
# For x86_64 Linux
curl -L -o titen https://github.com/codecoradev/titen/releases/latest/download/titen-linux-amd64

# For ARM64 Linux (e.g., Oracle Cloud ARM, Raspberry Pi)
curl -L -o titen https://github.com/codecoradev/titen/releases/latest/download/titen-linux-arm64

chmod +x titen
sudo mv titen /usr/local/bin/titen
```

### Option B: Build from Source

```bash
git clone https://github.com/codecoradev/titen.git
cd titen
cargo build --release
sudo cp target/release/titen /usr/local/bin/titen
```

### SQLite Database Location

By default, the binary stores the database at:

```
~/.codecora/titen/titen.db
```

Override with the `TITEN_DB_PATH` environment variable:

```bash
export TITEN_DB_PATH="/var/lib/titen/titen.db"
```

### Systemd Service File

Create the data directory and a dedicated user:

```bash
sudo useradd -r -s /bin/false titen
sudo mkdir -p /var/lib/titen
sudo chown titen:titen /var/lib/titen
```

Create `/etc/systemd/system/titen.service`:

```ini
[Unit]
Description=Titen: Self-hosted Threads Management Platform
Documentation=https://github.com/codecoradev/titen
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=titen
Group=titen

# Binary location
ExecStart=/usr/local/bin/titen serve

# Environment configuration
Environment="TITEN_DB_PATH=/var/lib/titen/titen.db"
Environment="TITEN_API_KEY=your-secure-api-key-here"
Environment="TITEN_HOST=0.0.0.0"
Environment="TITEN_PORT=7845"
Environment="TITEN_URL=https://titen.yourdomain.com"
Environment="TITEN_WEB_DIR=/usr/local/share/titen/web"
Environment="TITEN_CORS_ORIGINS=https://titen.yourdomain.com"
Environment="RUST_LOG=info"

# Security hardening
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/var/lib/titen
PrivateTmp=yes

# Restart policy
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

Enable and start the service:

```bash
sudo systemctl daemon-reload
sudo systemctl enable titen
sudo systemctl start titen

# Verify
sudo systemctl status titen
curl http://localhost:7845/api/health
```

> **Web assets:** If deploying natively, copy the built SvelteKit static files to `TITEN_WEB_DIR` (e.g., `/usr/local/share/titen/web`). Without these, the dashboard will not load (API still works).

---

## 5. Reverse Proxy Configuration

In production, Titen must be placed behind a reverse proxy for TLS termination and security.

### Caddy (recommended for automatic HTTPS)

Caddy is recommended because it **automatically provisions and renews TLS certificates** via Let's Encrypt.

Install Caddy: [https://caddyserver.com/docs/install](https://caddyserver.com/docs/install)

Create `/etc/caddy/Caddyfile`:

```caddyfile
titen.yourdomain.com {
    reverse_proxy localhost:7845

    # Allow large media uploads (images/video)
    request_body {
        max_size 100MB
    }

    # Pass real client IP
    header_up X-Real-IP {remote_host}
    header_up X-Forwarded-For {remote_host}
    header_up X-Forwarded-Proto {scheme}

    # Security headers
    header {
        Strict-Transport-Security "max-age=31536000; includeSubDomains"
        X-Content-Type-Options "nosniff"
        X-Frame-Options "DENY"
        Referrer-Policy "strict-origin-when-cross-origin"
    }

    # Compression
    encode gzip zstd
}
```

Restart Caddy:

```bash
sudo systemctl restart caddy
```

Caddy will automatically obtain a TLS certificate and redirect HTTP to HTTPS.

### Nginx

Install Nginx and obtain certificates (e.g., via `certbot`).

Create `/etc/nginx/sites-available/titen.conf`:

```nginx
server {
    listen 80;
    server_name titen.yourdomain.com;

    # Redirect all HTTP to HTTPS
    return 301 https://$host$request_uri;
}

server {
    listen 443 ssl http2;
    server_name titen.yourdomain.com;

    # TLS certificates (adjust paths to your certbot/cert location)
    ssl_certificate     /etc/letsencrypt/live/titen.yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/titen.yourdomain.com/privkey.pem;
    ssl_protocols       TLSv1.2 TLSv1.3;
    ssl_ciphers         HIGH:!aNULL:!MD5;
    ssl_prefer_server_ciphers on;

    # Allow large media uploads (images + video)
    client_max_body_size 100M;

    # Proxy to Titen
    location / {
        proxy_pass http://127.0.0.1:7845;
        proxy_http_version 1.1;

        # WebSocket support (for real-time features if used)
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";

        # Pass real client information
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Timeout settings for long-running requests (media processing)
        proxy_read_timeout 300s;
        proxy_send_timeout 300s;
    }

    # Security headers
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-Frame-Options "DENY" always;
    add_header Referrer-Policy "strict-origin-when-cross-origin" always;

    # Gzip compression
    gzip on;
    gzip_types text/css application/javascript application/json image/svg+xml;
    gzip_min_length 256;
}
```

Enable the site:

```bash
sudo ln -s /etc/nginx/sites-available/titen.conf /etc/nginx/sites-enabled/
sudo nginx -t          # Test config
sudo systemctl reload nginx
```

---

## 6. Threads API Setup

Titen uses the **Threads API** to publish and manage posts on behalf of your connected Threads accounts. This is separate from the admin login. It connects your Threads social account.

### Step 1: Create a Meta App

1. Go to [Meta for Developers](https://developers.facebook.com/)
2. Click **My Apps → Create App**
3. Choose **Business** as the app type
4. Fill in app name (e.g., "Titen") and contact email
5. Submit to create the app

### Step 2: Add Threads API

1. In your app dashboard, scroll to **Add Product**
2. Find **Threads API** and click **Set Up**
3. This adds the Threads API to your app

### Step 3: Configure OAuth

1. Under **Threads API → Settings**, note your:
   - **App ID**
   - **App Secret**
2. Set the **OAuth Redirect URI** to:
   ```
   https://titen.yourdomain.com/auth/callback
   ```
3. Add the required permissions:
   - `threads_basic` (read profile info)
   - `threads_content_publish` (publish posts)
   - `threads_manage_insights` (read analytics)
   - `threads_manage_reply` (manage replies)

### Step 4: Connect via OAuth Flow

Titen provides a built-in OAuth flow. Start it from the web dashboard or via the API:

**Option A, via API:**

```bash
# Initiate OAuth; returns a redirect URL
curl -X POST https://titen.yourdomain.com/api/threads/oauth/initiate \
  -H "X-API-Key: your-api-key"
```

**Option B, via Web Dashboard:**

1. Log in to your Titen dashboard at `https://titen.yourdomain.com/login`
2. Navigate to **Settings → Threads Accounts → Connect Account**
3. Click **Authorize with Threads**

**The flow:**

```
1. Initiate:  POST /api/threads/oauth/initiate
              → Returns redirect URL to Meta's authorization page

2. Authorize: User visits the URL → logs into Threads → grants permissions
              → Meta redirects to callback URL

3. Callback:  GET /auth/callback?code=...&state=...
              → Titen exchanges code for access token
              → Token is stored in the database
              → Account is connected and ready to post
```

### Step 5: Token Refresh

Threads API access tokens **expire** (typically 60 days). Titen handles refresh automatically:

**Automatic Refresh:**

The built-in scheduler checks token validity every 60 seconds (configurable via `TITEN_SCHEDULER_INTERVAL_SECS`) and refreshes tokens before they expire.

**Manual Refresh (CLI):**

```bash
# Inside the container
docker exec -it titen titen account list

# Refresh token manually
docker exec -it titen titen account refresh <account_id>
```

If a token expires, the Threads API will return `401 Unauthorized`. See [Troubleshooting](#11-troubleshooting).

---

## 7. S3 Media Storage (optional)

### When You Need It

S3-compatible storage is needed when you:

- **Upload images** (IMAGE post type)
- **Upload videos** (VIDEO post type)
- **Store media locally** is not sufficient for your scale

Without S3 configured, media uploads will fail. TEXT-only posts work without S3.

### Supported Providers

Titen works with any S3-compatible storage:

- **MinIO** (self-hosted, recommended for single-server deployments)
- **AWS S3**
- **Cloudflare R2**
- **Backblaze B2**
- **DigitalOcean Spaces**
- **Wasabi**

### Environment Variables

Configure all 6 `TITEN_S3_*` environment variables:

| Variable | Description | Example |
|---|---|---|
| `TITEN_S3_ENDPOINT` | S3 API endpoint URL | `https://s3.amazonaws.com` (AWS) or `http://minio:9000` (local MinIO) |
| `TITEN_S3_BUCKET` | Bucket name where media is stored | `titen-media` |
| `TITEN_S3_REGION` | AWS region (or `us-east-1` for most S3-compatible) | `us-east-1` |
| `TITEN_S3_ACCESS_KEY` | Access key ID for authentication | `your-access-key-id` |
| `TITEN_S3_SECRET_KEY` | Secret access key for authentication | `your-secret-access-key` |
| `TITEN_S3_PUBLIC_URL` | Public base URL for serving media to clients | `https://media.yourdomain.com` |

> **Public URL:** The `TITEN_S3_PUBLIC_URL` must be publicly accessible so that media can be fetched when publishing to Threads. If using MinIO behind a reverse proxy, point this to your MinIO public domain.

### MinIO Example (Docker Compose)

Add MinIO as a service alongside Titen:

```yaml
services:
  minio:
    image: minio/minio:latest
    container_name: minio
    restart: unless-stopped
    ports:
      - "127.0.0.1:9000:9000"  # API
      - "127.0.0.1:9001:9001"  # Console
    volumes:
      - minio-data:/data
    environment:
      MINIO_ROOT_USER: "minio-admin"
      MINIO_ROOT_PASSWORD: "your-minio-password"
    command: server /data --console-address ":9001"

  titen:
    # ... (as in Section 3)
    environment:
      # ... other vars ...
      TITEN_S3_ENDPOINT: "http://minio:9000"
      TITEN_S3_BUCKET: "titen-media"
      TITEN_S3_REGION: "us-east-1"
      TITEN_S3_ACCESS_KEY: "minio-admin"
      TITEN_S3_SECRET_KEY: "your-minio-password"
      TITEN_S3_PUBLIC_URL: "https://media.yourdomain.com"  # Via reverse proxy
    depends_on:
      - minio

volumes:
  titen-data:
  minio-data:
```

---

## 8. Security Checklist

Before going to production, verify every item on this checklist:

- [ ] **Set `TITEN_API_KEY`**: Mandatory for production. Without it, Titen runs in dev mode with all endpoints open and unauthenticated. Generate with `openssl rand -hex 32`.

- [ ] **Set `TITEN_ENCRYPTION_KEY`**: Generates the AES-256-GCM key for encrypting `access_token` and `app_secret` at rest. Without it, tokens are stored in plaintext. Generate with `openssl rand -hex 32`. Back up this key: losing it makes existing encrypted tokens unrecoverable.

- [ ] **Set `TITEN_REQUIRE_ENCRYPTION=true`**: Fail-fast guard. The server refuses to start if the encryption key is missing. Recommended for all production deployments.

- [ ] **Behind TLS reverse proxy**: Titen does not handle TLS itself. Use Caddy or Nginx. The `Secure` flag on session cookies requires HTTPS.

- [ ] **Firewall: only expose 443/80**: Do NOT expose port 7845 directly to the internet. The Docker Compose example binds to `127.0.0.1:7845` to prevent direct access.

  ```bash
  # UFW example
  sudo ufw allow 22/tcp
  sudo ufw allow 80/tcp
  sudo ufw allow 443/tcp
  sudo ufw deny 7845/tcp
  sudo ufw enable
  ```

- [ ] **Set `TITEN_CORS_ORIGINS`**: Restrict cross-origin policy to your exact domain (e.g., `https://titen.yourdomain.com`). Do not leave this unset in production.

- [ ] **Volume backup strategy for `/data`**: Set up automated backups (see [Section 9](#9-backup--recovery)). The SQLite database and uploaded media live here.

- [ ] **Regular SQLite backup**: Use the safe online backup command (does not lock the database):
  ```bash
  sqlite3 /data/titen.db ".backup '/data/backup.db'"
  ```

- [ ] **Keep Docker image updated**: Subscribe to release notifications and update regularly:
  ```bash
  docker compose pull && docker compose up -d
  ```

- [ ] **Protect `TITEN_API_KEY`**: Store it in a `.env` file (not committed to git) or a secrets manager. Never hardcode in version control.

- [ ] **Restrict Threads OAuth redirect URI**: Ensure only your production domain is registered in the Meta for Developers console.

---

## 9. Backup & Recovery

### SQLite Backup (Online / Hot Backup)

This command creates a safe backup without locking the running database:

```bash
# Docker
docker exec titen sqlite3 /data/titen.db ".backup '/data/titen-backup-$(date +%Y%m%d).db'"

# Native
sqlite3 /var/lib/titen/titen.db ".backup '/var/lib/titen/titen-backup-$(date +%Y%m%d).db'"
```

### Automated Backup Script

Create a daily cron job:

```bash
# /opt/titen/backup.sh
#!/bin/bash
BACKUP_DIR="/opt/backups/titen"
mkdir -p "$BACKUP_DIR"

# Backup SQLite
docker exec titen sqlite3 /data/titen.db ".backup '/data/backup.db'"
docker cp titen:/data/backup.db "$BACKUP_DIR/titen-$(date +%Y%m%d-%H%M%S).db"

# Backup full volume
docker run --rm -v titen_titen-data:/data -v "$BACKUP_DIR":/backup \
  alpine tar czf "/backup/titen-volume-$(date +%Y%m%d-%H%M%S).tar.gz" /data

# Retain last 30 days
find "$BACKUP_DIR" -name "titen-*.db" -mtime +30 -delete
find "$BACKUP_DIR" -name "titen-volume-*.tar.gz" -mtime +30 -delete

echo "Backup complete: $(date)"
```

```bash
chmod +x /opt/titen/backup.sh

# Add to crontab; runs daily at 3 AM
crontab -e
# 0 3 * * * /opt/titen/backup.sh >> /var/log/titen-backup.log 2>&1
```

### Volume Backup with Docker

Back up the entire data volume (SQLite + media + config):

```bash
# Create backup
docker run --rm -v titen_titen-data:/data -v $(pwd):/backup \
  alpine tar czf /backup/titen-volume-$(date +%Y%m%d).tar.gz /data

# List backups
ls -lh titen-volume-*.tar.gz
```

### Restore Procedure

#### Restore SQLite Database

```bash
# 1. Stop Titen
docker compose down

# 2. Copy backup into the volume
docker run --rm -v titen_titen-data:/data -v $(pwd):/backup \
  alpine cp /backup/titen-backup-20260101.db /data/titen.db

# 3. Start Titen
docker compose up -d

# 4. Verify
curl http://localhost:7845/api/health
```

#### Restore Full Volume

```bash
# 1. Stop Titen
docker compose down

# 2. Extract volume backup
docker run --rm -v titen_titen-data:/data -v $(pwd):/backup \
  alpine tar xzf /backup/titen-volume-20260101.tar.gz -C /

# 3. Start Titen
docker compose up -d

# 4. Verify
docker logs titen
curl http://localhost:7845/api/health
```

---

## 10. Updating

### Docker

Updating is a single command sequence:

```bash
# Pull the latest image
docker compose pull

# Recreate the container with the new image
docker compose up -d

# Verify the new version is running
docker logs titen
curl http://localhost:7845/api/health
```

This preserves your data volume. No data loss.

### Binary (Native Deploy)

```bash
# 1. Download the new binary
curl -L -o titen-new https://github.com/codecoradev/titen/releases/latest/download/titen-linux-amd64
chmod +x titen-new

# 2. Stop the service
sudo systemctl stop titen

# 3. Replace the binary
sudo mv titen-new /usr/local/bin/titen

# 4. Start the service
sudo systemctl start titen

# 5. Verify
sudo systemctl status titen
curl http://localhost:7845/api/health
```

### Database Migration

Titen includes **built-in database migrations** (4 migrations as of current release). Migrations run **automatically on startup**. No manual action needed.

On startup, Titen will:

1. Check the current database schema version
2. Apply any pending migrations
3. Log the migration status

```
# Example startup log
INFO  titen > Running database migrations...
INFO  titen > Migration 001_create_tables: applied
INFO  titen > Migration 002_add_scheduler: applied
INFO  titen > Migration 003_add_video_posts: applied
INFO  titen > All migrations applied successfully.
```

> **Always back up before updating.** See [Section 9](#9-backup--recovery).

---

## 11. Troubleshooting

### Port Already in Use

**Symptom:** Container fails to start with `Address already in use` or `bind: address already in use`.

**Diagnosis:**

```bash
# Check what's using port 7845
sudo lsof -i :7845
# or
sudo ss -tlnp | grep 7845
```

**Fix:**

```bash
# Option 1: Stop the conflicting process
sudo kill -9 <PID>

# Option 2: Change Titen's port
# In docker-compose.yml:
#   ports:
#     - "127.0.0.1:7850:7845"  # Map to a different host port
# Or set TITEN_PORT in the environment
```

---

### SQLite Database Locked

**Symptom:** API returns `500 Internal Server Error` with log message `database is locked`.

**Causes & Fixes:**

1. **Multiple instances accessing the same database**: ensure only one Titen instance is running against the same SQLite file.

   ```bash
   # Check for duplicate containers
   docker ps | grep titen
   ```

2. **Stale lock from a crashed process**: restart the container:

   ```bash
   docker compose restart titen
   ```

3. **Disk full**: check disk space:

   ```bash
   df -h
   docker system df
   ```

4. **WAL file corruption**: if persistent, run a checkpoint:

   ```bash
   docker exec titen sqlite3 /data/titen.db "PRAGMA wal_checkpoint(TRUNCATE);"
   ```

---

### Threads API 401 (Expired Token)

**Symptom:** Scheduled posts fail with `401 Unauthorized` from Threads API.

**Diagnosis:**

```bash
# Check account status
docker exec titen titen account list

# View recent errors in logs
docker logs titen 2>&1 | grep -i "401\|unauthorized\|token"
```

**Fix:**

1. **Manual refresh:**

   ```bash
   docker exec -it titen titen account refresh <account_id>
   ```

2. **Re-authenticate if refresh fails** (token fully expired):

   ```bash
   # Re-initiate OAuth flow
   curl -X POST https://titen.yourdomain.com/api/threads/oauth/initiate \
     -H "X-API-Key: your-api-key"
   ```

   Follow the redirect URL to re-authorize the account.

3. **Verify the scheduler is running** (handles automatic refresh):

   ```bash
   docker logs titen 2>&1 | grep -i "scheduler\|refresh\|token"
   ```

---

### Media Upload Fails (S3 Not Configured)

**Symptom:** Uploading images or videos returns an error; TEXT posts work fine.

**Diagnosis:**

```bash
# Check logs for S3 errors
docker logs titen 2>&1 | grep -i "s3\|media\|upload"
```

**Fix:**

Ensure all 6 `TITEN_S3_*` environment variables are set (see [Section 7](#7-s3-media-storage-optional)):

```yaml
environment:
  TITEN_S3_ENDPOINT: "https://your-s3-endpoint"
  TITEN_S3_BUCKET: "titen-media"
  TITEN_S3_REGION: "us-east-1"
  TITEN_S3_ACCESS_KEY: "your-key"
  TITEN_S3_SECRET_KEY: "your-secret"
  TITEN_S3_PUBLIC_URL: "https://your-public-url"
```

Then restart:

```bash
docker compose up -d
```

---

### CORS Errors

**Symptom:** Browser console shows `Access-Control-Allow-Origin` errors; the dashboard can't reach the API.

**Diagnosis:**

```
# Browser DevTools Console:
# "Access to fetch at 'https://titen.yourdomain.com/api/...' from origin 'https://titen.yourdomain.com'
#  has been blocked by CORS policy: No 'Access-Control-Allow-Origin' header..."
```

**Fix:**

Set `TITEN_CORS_ORIGINS` to your exact domain (scheme + host, no trailing slash):

```yaml
environment:
  TITEN_CORS_ORIGINS: "https://titen.yourdomain.com"
```

For multiple origins (comma-separated):

```yaml
environment:
  TITEN_CORS_ORIGINS: "https://titen.yourdomain.com,https://admin.yourdomain.com"
```

Restart after changing:

```bash
docker compose up -d
```

---

### Container Won't Start

**Symptom:** `docker compose up -d` exits immediately or container status is `Exited`.

**Diagnosis:**

```bash
# View container logs
docker logs titen

# Check container status
docker compose ps -a

# Inspect exit code
docker inspect titen --format='{{.State.ExitCode}}'
```

**Common causes:**

| Exit Code | Cause | Fix |
|---|---|---|
| `1` | Application error | Read logs for specific error |
| `137` | Out of memory (OOM killed) | Increase memory limit or check for leaks |
| `139` | Segfault (binary/arch mismatch) | Ensure correct image for your architecture (amd64 vs arm64) |

**Step-by-step:**

```bash
# 1. Read the full logs
docker logs titen --tail 100

# 2. Check if port is in use
docker compose down
sudo ss -tlnp | grep 7845

# 3. Verify the image pulled correctly
docker compose pull

# 4. Check disk space
df -h

# 5. Start with foreground to see real-time output
docker compose up  # (without -d)
```

---

### Getting Help

If you're still stuck:

- **GitHub Issues:** [github.com/codecoradev/titen/issues](https://github.com/codecoradev/titen/issues)
- **Documentation:** [github.com/codecoradev/titen/docs](https://github.com/codecoradev/titen/tree/main/docs)
- **Logs:** Always include `docker logs titen` output when reporting issues
