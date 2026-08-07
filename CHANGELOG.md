# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- CONTRIBUTING.md with full contribution guide, branch strategy, CI checks, and architecture overview
- SECURITY.md with vulnerability reporting policy and supported versions table
- CODE_OF_CONDUCT.md
- PR template with Conventional Commits checklist, Rust + frontend testing checklist
- Structured issue templates (YAML form-based bug report and feature request)
- `TITEN_ENCRYPTION_KEY` env var for AES-256-GCM token encryption at rest

### Security
- Encrypt `access_token` and `app_secret` at rest with AES-256-GCM (PR #48, closes #47)
- Per-encryption random 96-bit nonce via `rand::thread_rng()`
- `enc:v1:` versioned prefix for transparent migration of existing plaintext data
- Migration 004: one-time plaintext-to-encrypted upgrade on startup (idempotent)
- Encryption key zeroized on drop via `zeroize` crate

## [0.2.0] - 2026-08-05

Docker GHCR images + scheduler hardening + two-container architecture.

### Added
- Two-container Docker setup: `web` (Bun + SvelteKit SSR) + `api` (Rust Axum, alpine:3.22)
- GHCR Docker workflow with native ARM runner (split per-arch + manifest merge)
- `docker-compose.yml` with Traefik optional + GHCR image pull support
- CAROUSEL media type: multi-step flow (N children containers → publish carousel)
- `reap_stale_schedules()` — reaps schedules stuck in `processing` > 5min timeout
- `TITEN_COOKIE_SECURE` env var for production HTTPS Secure flag
- `.env.example` with all configuration documented

### Fixed
- Scheduler race condition: separate SELECT+UPDATE replaced with atomic UPDATE...WHERE status='scheduled'
- Cookie `Secure` attribute: omit entirely when `TITEN_COOKIE_SECURE=false` (not `Secure=false` literal)
- Admin auth guard: async session check with loading state + error fallback

## [0.1.0] - 2026-07-19
