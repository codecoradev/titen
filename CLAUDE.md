# titen — CLAUDE.md

## Project Overview
Self-hosted Threads management platform. Rust (Axum + SQLx + SQLite), Docker-ready.

## Conventions
- **Edition**: Rust 2024, minimum 1.88
- **Binary names**: `titen` (CLI), `titen-api` (HTTP), `titen-mcp` (MCP)
- **Crates**: `titen-core` (domain), `titen-api` (HTTP), `titen-cli` (CLI), `titen-mcp` (MCP)
- **DB**: SQLite via SQLx, migrations in `crates/titen-api/migrations/`
- **Config**: ENV-driven, `TITEN_` prefix. No config files.
- **Error handling**: `titen_core::TitenError` (thiserror), propagates as `Result<T>`
- **IDs**: UUID v7 via `uuid` crate
- **HTTP**: Axum, JSON responses, `axum::http::StatusCode` for status codes

## Build
```bash
cargo build --release          # all crates
cargo build -p titen-api      # HTTP server only
cargo build -p titen-cli       # CLI only
cargo build -p titen-mcp      # MCP only
```

## Test
```bash
cargo test --workspace
```

## Lint
```bash
cargo fmt -- --check
cargo clippy --workspace -- -D warnings
```

## Design Decisions
- Scheduling is local (tokio-cron-scheduler) because Threads API has no `publish_at`
- Sentiment engine is pluggable (trait-based): stub → ONNX → LLM → custom API
- S3 storage is optional, generic (MinIO/R2/etc), via ENV
- Rate limiting uses sliding 24h window per account per action

## Architecture
See DESIGN.md for full specification including API routes, CLI commands, MCP tools, DB schema.
