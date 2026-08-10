# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.0] - 2026-08-10

A security and reliability focused release. This version hardens the production attack surface, encrypts stored settings, and cleans up a large backlog of frontend consistency issues.

### Added

- **Carousel thumbnail preview in schedule list.** Scheduled posts that include multiple media items now display a visual thumbnail strip, making it easier to distinguish carousel posts at a glance. [#76]
- **Location tagging for scheduled posts.** Posts can now carry location metadata through the full publish flow. [#68]
- **Encrypted application settings.** Instance-wide configuration (API keys, webhook secrets) moved from browser localStorage to the server-side SQLite database with AES-256-GCM encryption at rest.

### Changed

- **UI component standardization.** All admin interfaces migrated to shadcn-svelte primitives (Dialog, Table, Select, Badge). Removes ad-hoc CSS components in favor of a single, maintained design system.

### Fixed

- **Security hardening (PR #142, #143, #144).** Production attack surface reduced through multiple fixes:
  - Session tokens are now 256-bit opaque hex strings — no longer derivable from account identity.
  - Rate limiting migrated from `Mutex<HashMap>` to `DashMap` for lock-free throughput under load.
  - Media downloads switched from direct S3 reads to time-limited presigned URLs.
  - List endpoints now enforce pagination caps to prevent unbounded query results.
  - SigV4 canonical query string encoding made consistent across upload and download paths.
- **Token drift and reactive loop bugs (PR #134).** Frontend `$effect` loops, analytics math errors, and an XSS vector in media rendering were resolved.
- **Caption length validation (PR #137).** The API now rejects captions exceeding 500 characters before they reach Threads, matching the platform limit. [#136]
- **Frontend consistency cleanup (PR #138, #145).** Consolidated duplicate modal CSS into shared global styles, removed 33 inline `style` declarations in favor of utility classes, fixed mismatched `form-hint`/`form-helper` naming, and standardized button class usage. [#125–#132]
- **Eight CSS/UX issues resolved (PR #138).** Backdrop blur on confirm dialogs, image fallback handlers, and responsive layout fixes across admin pages.

## [0.5.5] - 2026-08-09

### Fixed

11 bugs across scheduler, store, and Threads client — all validated via DB-level integration testing (13/13 tests pass).

- **Timezone mismatch in scheduler query**: `get_due_schedules` compared raw ISO timestamps with `datetime('now')`, causing timezone-aware schedules (e.g. `+07:00`) to be missed or delayed. Fixed to use `datetime(scheduled_at)` for consistent comparison. [#107]
- **`threads_post_id` not persisted on publish**: Both the immediate-publish path (`POST /api/posts`) and the scheduler path silently dropped the Threads API post ID, making it impossible to delete or manage published posts. Fixed by using `create_post_with_threads_id()`. [#106, #109]
- **`delete_post()` leaked access token in query string**: The Threads API `DELETE` call passed the access token as a URL query parameter (`.query(&body)`) instead of the `Authorization` header. Fixed to use header-only auth. [#115]
- **`user_id` derivation via fragile token parsing**: The `/me` lookup split the access token on `|` and took the first segment — a brittle heuristic that breaks with non-standard token formats. Replaced with a fallback to the Graph API `/me` endpoint. [#116]
- **`published_at` and `result_post_id` never set**: `update_schedule_status()` updated only the `status` column, leaving `published_at` and `result_post_id` null even after successful publishing. Fixed to set both fields when status transitions to `published`. [#112]
- **Schedule edit caused data loss (delete + recreate)**: The `PUT /api/schedules/{id}` handler deleted the schedule and recreated it, losing `created_at`, `result_json`, and audit metadata. Replaced with a direct `UPDATE` that preserves all fields. [#113]
- **Schedule editable in `pending` state**: Schedules could be edited after approval (`pending`), creating a race condition with the scheduler. Restricted to `draft` state only. [#114]
- **`list_upcoming` included past schedules**: The "upcoming" endpoint returned all pending schedules regardless of timestamp, including ones already past due. Fixed to filter on `scheduled_at > now`. [#117]
- **Inactive accounts still processed by scheduler**: The scheduler tick did not check `accounts.is_active`, attempting to publish for deactivated accounts. Fixed to skip inactive accounts. [#117]
- **PUT handler always returned HTTP 200**: Error responses (conflict, not found) were returned with HTTP 200 status. Fixed to return correct status codes (409, 404, 500). [#122]
- **PUT handler silently dropped `text_attachment`**: `CreateSchedule` has both `caption` and `text_attachment`, but the PUT handler only read `caption`. Fixed with `caption.or(text_attachment)` merge. [#122]

## [0.5.4] - 2026-08-09

### Added
- **Local filesystem storage fallback**: media uploads now work out-of-the-box without S3/MinIO configured. Files are saved to `TITEN_LOCAL_STORAGE_DIR` (default: `/data/media`) and served via `/media/` route. Path traversal protection included. [#105]
  - Auto-detection: S3 takes priority when `TITEN_S3_ENDPOINT` is set, otherwise local filesystem
  - Docker bind volume `./data:/data` covers persistence — no extra config needed

### Fixed
- **SigV4 canonical URI missing leading slash**: S3 uploads returned `SignatureDoesNotMatch` because the canonical URI path was signed without a leading `/` (e.g. `hermes/2026/...` instead of `/hermes/2026/...`). [#104]

## [0.5.3] - 2026-08-09

### Added
- **Interactive API docs (Swagger UI)**: all endpoints are now self-documenting at `/api/docs` with a full OpenAPI 3.0 spec at `/api/docs/openapi.json`. Powered by `utoipa` + `utoipa-swagger-ui`. [#85]
  - 21 paths, 34 schema definitions, 7 tags (health, accounts, posts, schedules, comments, analytics, media)
  - API key security scheme documented — click "Authorize" in Swagger UI to test authenticated endpoints

### Fixed
- **Migration 008 registration**: `comment_reply_status.sql` was created in #99 but never registered in `store.migrate()`, causing `no such column: reply_status` errors on fresh databases. [#100]

## [0.5.2] - 2026-08-08

### Added
- **13 new MCP tools**: expanded MCP handler from 17 → 30 tools, covering all major API capabilities previously accessible via REST. [#84]
  - `get_post`, `get_schedule`, `approve_schedule`, `reject_schedule` — schedule/post lifecycle
  - `list_media`, `upload_media` — media management with S3 upload
  - `fetch_mentions`, `list_mentions` — mention monitoring (API fetch + DB list)
  - `search_keyword` — keyword search across Threads
  - `get_post_trend` — analytics trend per post
  - `reply_to_comment` — reply to Threads comments
  - `exchange_oauth_code`, `create_account` — OAuth flow + account onboarding
- MCP `Cargo.toml`: added `reqwest` and `chrono` workspace dependencies for HTTP media download and date handling.

### Fixed
- **SSRF protection on `upload_media`**: all outbound HTTP requests now enforce `redirect(Policy::none())` (no redirect following) and `is_private_host()` DNS check before download — blocks requests to `127.0.0.0/8`, `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `169.254.0.0/16`, IPv6 `::1`, `fc00::/7` (ULA), and `fe80::/10` (link-local).
- **Fail-closed DNS resolution**: `is_private_host()` now returns `true` (unsafe) on DNS resolution errors and empty address iterators, preventing fail-open bypasses.
- **File type validation via magic bytes**: `validate_magic_bytes()` checks actual file signatures (JPEG, PNG, WebP, GIF) with per-format length guards — rejects spoofed extensions.
- **Schema/handler field alignment**: `upload_media` schema now includes `filename` (handler already reads it); `list_media` handler now reads `media_type` to match its schema (was reading non-existent `content_type`).
- **Chunked encoding OOM bypass**: removed redundant `Content-Length` pre-check in favor of streaming body with hard 50 MB cap via `resp.chunk()` — handles both Content-Length and chunked transfer encoding uniformly.
- **Hardcoded token expiry removed**: `exchange_oauth_code` now performs full short-lived → long-lived token exchange to get actual `expires_in` from the Threads API instead of assuming a hardcoded 5,184,000-second (60-day) value.

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
