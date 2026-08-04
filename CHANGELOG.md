# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- VIDEO media type support across API, scheduler, and Threads client
- `claim_schedule()` atomic claim to prevent double-posting in HA setups
- `publish_video()` method with container status polling (max ~4.5min timeout)
- Cookie-based session auth (httpOnly, SameSite=Strict)
- CORS hardening with `TITEN_CORS_ORIGINS` env var

### Fixed
- Scheduler race condition: separate SELECT+UPDATE replaced with atomic UPDATE...WHERE status='pending'
- CORS origin `unwrap()` panic on malformed URL — now silently skipped
- localStorage API key exposure removed — cookie-only auth
- Admin auth guard: async session check with loading state + error fallback
- Dockerfile stage numbering (Stage 2 → Stage 3)
- `list_accounts` now uses `safe_account_json` for consistent field output (adds `updated_at`)
