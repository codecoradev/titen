# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.1] - 2026-08-08

### Added
- **Unified query/filter system**: all list endpoints now support structured filtering via reusable filter structs (`PostFilter`, `ScheduleFilter`, `CommentFilter`, `MediaFilter`, `MentionFilter`, `AccountFilter`). Each supports date range (`from`/`to`), text search, entity-specific fields (e.g. `media_type`, `sentiment`), and pagination (`limit` clamped 1–1000, `offset` defaults 0). [#83]
- **Dynamic SQL builder**: store layer now constructs WHERE clauses dynamically based on provided filter fields — no more hardcoded query params.
- **Pagination on all list endpoints**: `list_schedules`, `list_comments`, `list_media`, and `list_mentions` now support `limit`/`offset` (previously missing or limited).

### Changed
- **Breaking (API query params)**: `GET /api/posts` now accepts `from`/`to` instead of `start_date`/`end_date`. `GET /api/schedules` now accepts `limit`/`offset` in addition to `account_id`/`status`. `GET /api/media` now accepts `account_id`/`media_type`/`limit`/`offset`.
- **MCP tool signatures**: `list_schedules`, `list_posts`, `list_comments` MCP handlers updated to pass filter structs.
- **Store function signatures**: `list_posts`, `list_schedules`, `list_comments`, `list_media`, `list_mentions` now take a single `&Filter` parameter instead of individual arguments.

## [0.5.0] - 2026-08-08

### Added
- **Mentions persistence (P0)**: mentions from Threads API are now persisted to a dedicated `mentions` table, preventing data loss on every fetch. New endpoints: `GET /api/threads/mentions` (list from DB) and `POST /api/threads/mentions` (fetch + persist from API). [#78]
- **GET /api/schedules/{id}**: single schedule retrieval via GET method (previously only PUT/PATCH/DELETE). [#81]
- **Caption sanitizer**: multiline captions are now normalized (`\r\n` → `\n`, null bytes stripped) on both create and update paths, preventing broken JSON parsing from raw curl. [#82]
- Migration `006_mentions_table.sql`: new `mentions` table with `account_id` FK, `threads_mention_id` unique constraint, and full mention metadata.
- Migration `007_media_urls_doc.sql`: marker migration documenting `media_urls` TEXT column as JSON-encoded array convention. [#80]

### Changed
- **Schedule.media_urls documentation**: added rustdoc clarifying that `media_urls` is a TEXT column storing a JSON-encoded array (SQLite has no native JSON type). Consumer code uses `serde_json::from_str` to decode. [#80]

## [0.4.2] - 2026-08-08

### Fixed
- **Hamburger button invisible on mobile**: removed `btn-ghost` class (transparent bg + muted color = invisible). Now has standalone styling with visible background, border, and fixed 2.5rem size.
- **Dashboard header overlap**: mobile content padding adjusted to `calc(2.5rem + var(--space-md) + var(--space-sm))` so content clears the fixed hamburger button.

## [0.4.1] - 2026-08-08

### Fixed
- **Sidebar version display**: hardcoded `v0.3.0` → correct `v0.4.0`
- **Insights dropdown**: used non-existent `.select` CSS class → switched to `.form-input` (proper styling)
- **Dashboard grid responsive**: fragile `div[style*="grid-template-columns"]` attribute selector → proper `.dashboard-grid` class

### Changed
- **Mobile sidebar overlay**: click-outside backdrop to close sidebar (standard mobile pattern)
- **Mobile content padding**: top offset so hamburger button doesn't overlap page content
- **Insights header responsive**: stacks vertically + select full-width on screens ≤480px
- **Token & compact rows**: flex-wrap and column layout on narrow screens
- **Toast container**: max-width constrained to viewport to prevent overflow
- **Stat cards**: subtle border + shadow hover transition for interactive feel
- **Section headings**: `.section-heading` class replaces repeated inline styles (4 occurrences)
- **Accessibility**: `aria-expanded` attribute on hamburger menu button

## [0.4.0] - 2026-08-08

### Added
- **Human-in-the-Loop (HITL) scheduling**: new schedules default to `draft` status — they will NOT auto-publish until a human reviews and approves them
- **Approve workflow**: `POST /api/schedules/{id}/approve` transitions a draft to `pending` (ready for scheduler)
- **Reject workflow**: `POST /api/schedules/{id}/reject` transitions a draft to `rejected` with optional reason
- **Schedule editing**: `PATCH /api/schedules/{id}` allows editing caption, media_type, media_urls, and scheduled_at on drafts and pending items
- **Auto-approve flag**: `CreateSchedule.auto_approve` (default: `false`) — set to `true` to skip draft and go straight to pending (backward-compatible for API integrations)
- **Audit columns**: `approved_by` and `approved_at` columns track who approved each schedule and when
- Dashboard now shows a "Drafts (Needs Review)" stat card when drafts exist
- Schedules page shows draft count badge and contextual action buttons (Approve, Edit, Reject for drafts; Edit, Cancel for pending)

### Changed
- **New schedule lifecycle**: `draft → pending (approved) → processing → published/failed` — replaces the old `pending → processing → published/failed` where pending meant both "scheduled" and "ready"
- Existing schedules from pre-0.4.0 with `pending` status remain `pending` — they will continue to auto-publish normally
- Status filter dropdown updated with new states: Draft, Pending (Approved), Rejected
- MCP tool `schedule_post` defaults to `auto_approve: false` — MCP-created schedules also start as drafts

### Migration
- `005_hitl_scheduling.sql` adds `approved_by` and `approved_at` columns to the `schedules` table

## [0.3.0] - 2026-08-07

### Added
- **Account insights endpoint**: `GET /api/accounts/{id}/insights` exposes aggregate metrics (views, likes, replies, reposts, quotes, followers_count) from the Threads API — filterable via `?metrics=`, `?since=`, `?until=` query params
- **Mentions page**: new admin route `/admin/mentions` — fetch posts where your account is mentioned and reply directly from the UI
- **Insights dashboard panel**: dashboard now shows a per-account insights grid with account selector dropdown
- **Carousel from media library**: `CreatePost` accepts `media_ids: Vec<String>` — resolves media asset IDs to S3 URLs automatically for carousel posts
- **Reply from mentions**: mentions page includes a reply modal with inline text input
- Nav sidebar now includes "Mentions" link

### Changed
- **Token auto-refresh on publish**: `process_due_schedules` now checks token validity before each publish attempt — if token is `expired` or `expiring_soon`, it refreshes automatically instead of failing the schedule
- `CreatePost` struct extended with `media_ids`, `image_urls`, `video_url`, `alt_text` fields
- API client (`api.ts`) now exports `fetchMentions`, `createReply`, `getAccountInsights`, and extended `createPost` with all new fields

### Fixed
- **Token refresh gap**: scheduler's `check_all_tokens()` already refreshed every 6h, but per-post publishing had no refresh check — schedules would fail if token expired between ticks. Now each publish attempt ensures a valid token first

## [0.2.6] - 2026-08-07

### Fixed
- **Systemic HTTP error handling**: all Threads API calls now check HTTP status before parsing response body. Previously, non-2xx responses (400, 401, 502, etc.) were silently parsed as JSON, producing cryptic errors or crashes instead of the actual API error message
- **Publishing limit deserialize**: `get_publishing_limit()` now correctly extracts `data[0]` from the Threads API response (`{"data": [{...}]}`) instead of trying to deserialize the wrapper object directly
- **`create_container` / `publish_container`**: return the real Threads API error on failure instead of crashing on `.json()` parse of an error response — this was the root cause of HTTP 502 when publishing posts
- **`delete_post`**: now checks response status instead of silently succeeding on error
- Added `threads_get()` and `threads_post()` helper methods that centralize HTTP status checking and error message extraction for all Threads Graph API calls

## [0.2.5] - 2026-08-07

### Fixed
- Threads API error responses (e.g. expired code, invalid credentials) now properly parsed and surfaced to the user instead of showing empty "API 400:" message
- `ApiError` constructor now passes the real error message to `Error.message` instead of ignoring it
- OAuth code exchange now checks for `{"error": {...}}` response from Threads API and returns the actual error message (message, type, code) instead of generic "No access_token" fallback

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
