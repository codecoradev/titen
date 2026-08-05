# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-08-05

First stable release. Two-container Docker architecture, multi-platform GHCR images.

### Added
- Two-container Docker setup: `web` (Bun + SvelteKit SSR) + `api` (Rust Axum, alpine:3.22)
- GHCR Docker workflow with native ARM runner (split per-arch + manifest merge)
- `docker-compose.yml` with Traefik optional + GHCR image pull support
- VIDEO media type support across API, scheduler, and Threads client
- CAROUSEL media type: multi-step flow (N children containers → publish carousel)
- `claim_schedule()` atomic claim to prevent double-posting in HA setups
- `reap_stale_schedules()` — reaps schedules stuck in `processing` > 5min timeout
- `publish_video()` method with container status polling (max ~4.5min timeout)
- Cookie-based session auth (httpOnly, SameSite=Strict)
- `TITEN_COOKIE_SECURE` env var for production HTTPS Secure flag
- CORS hardening with `TITEN_CORS_ORIGINS` env var
- `.env.example` with all configuration documented

### Fixed
- Scheduler race condition: separate SELECT+UPDATE replaced with atomic UPDATE...WHERE status='scheduled'
- CORS origin `unwrap()` panic on malformed URL — now silently skipped
- localStorage API key exposure removed — cookie-only auth
- Admin auth guard: async session check with loading state + error fallback
- Dockerfile stage numbering (Stage 2 → Stage 3)
- `list_accounts` now uses `safe_account_json` for consistent field output (adds `updated_at`)
- Cookie `Secure` attribute: omit entirely when `TITEN_COOKIE_SECURE=false` (not `Secure=false` literal)
