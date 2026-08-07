# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.4] - 2026-08-07

### Fixed
- OAuth callback stuck at "Connecting your Threads account..." — `$effect` reactive loop when `goto()` triggers `$page` re-evaluation, causing `handleCallback()` to race with itself
- Added `hasRun` guard to ensure single execution, switched to `window.location.href` for hard redirect

## [0.2.3] - 2026-08-07

### Added
- Detailed OAuth callback tracing across all layers (frontend, SSR proxy, backend auth, OAuth exchange) to diagnose session loss after Threads approval

## [0.2.2] - 2026-08-07

### Fixed
- OAuth callback logout: the callback page checked `session.requires_auth` (always true in production) instead of `session.authenticated`, causing users to be redirected to `/login` after Threads approval (PR #59)
- Session cookie `SameSite=Strict` blocked the browser from sending the cookie on cross-site redirects, so the OAuth callback from Threads saw no session. Changed to `SameSite=Lax` which still prevents CSRF on POST but allows top-level GET navigations (PR #59)

## [0.2.1] - 2026-08-07

### Fixed
- Login redirect loop: `/api/auth/session` now validates the session cookie and returns an `authenticated` field, so the admin layout no longer bounces logged-in users back to `/login` (PR #57)
- Missing `TITEN_DB_PATH` in `.env.example` caused SQLite "unable to open database file" errors on fresh Docker deploys (PR #56)
- Docker deployment guide rewritten for two-container setup with correct `mkdir data/` step (PR #56)

### Security
- Encrypt `access_token` and `app_secret` at rest with AES-256-GCM (PR #48, closes #47)
- Per-encryption random 96-bit nonce via `rand::thread_rng()`
- `enc:v1:` versioned prefix for transparent migration of existing plaintext data
- Migration 004: one-time plaintext-to-encrypted upgrade on startup (idempotent)
- Encryption key zeroized on drop via `zeroize` crate
- `TITEN_REQUIRE_ENCRYPTION` env var: fail-fast on startup if encryption key is missing in production (PR #48)
- HTTP client timeouts on all outbound calls: Threads API (30s total, 10s connect) and S3 storage (60s total, 10s connect) (PR #50)
- `Arc::from_static` replaced raw static references to prevent pointer lifetime issues (PR #50)
- Error-swallowing patterns replaced with logged warnings: best-effort operations now surface failures instead of silently dropping them (PR #51)
- Token logging audit: confirmed no access tokens or secrets appear in log output at any level (PR #51)
- Integration test verifying encryption at rest: creates an account, reads raw SQLite, confirms ciphertext prefix `enc:v1:` is present and plaintext is absent (PR #52)

### Added
- CONTRIBUTING.md with full contribution guide, branch strategy, CI checks, and architecture overview
- SECURITY.md with vulnerability reporting policy and supported versions table
- CODE_OF_CONDUCT.md
- PR template with Conventional Commits checklist, Rust + frontend testing checklist
- Structured issue templates (YAML form-based bug report and feature request)
- `TITEN_ENCRYPTION_KEY` env var for AES-256-GCM token encryption at rest
- S3 storage and scheduler interval env vars in `.env.example` (PR #53)
- All encryption, cookie, scheduler, and S3 env vars passed through to API container in docker-compose.yml (PR #55)

### Documentation
- Encryption at rest documented in architecture.md, auth-flow.md, and README.md (PR #54)
- Database schema updated to 8 tables and 4 migrations (PR #54)

## [0.2.0] - 2026-08-05

Docker GHCR images + scheduler hardening + two-container architecture.

### Added
- Two-container Docker setup: `web` (Bun + SvelteKit SSR) + `api` (Rust Axum, alpine:3.22)
- GHCR Docker workflow with native ARM runner (split per-arch + manifest merge)
- `docker-compose.yml` with Traefik optional + GHCR image pull support
- CAROUSEL media type: multi-step flow (N children containers → publish carousel)
- `reap_stale_schedules()` reaps schedules stuck in `processing` beyond a 5-minute timeout
- `TITEN_COOKIE_SECURE` env var for production HTTPS Secure flag
- `.env.example` with all configuration documented

### Fixed
- Scheduler race condition: separate SELECT+UPDATE replaced with atomic UPDATE...WHERE status='scheduled'
- Cookie `Secure` attribute: omit entirely when `TITEN_COOKIE_SECURE=false` (not `Secure=false` literal)
- Admin auth guard: async session check with loading state + error fallback

## [0.1.0] - 2026-07-19
