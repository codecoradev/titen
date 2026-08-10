# Contributing to Titen

Terima kasih sudah ingin berkontribusi! Dokumen ini menjelaskan cara setup development, coding standards, dan workflow PR.

## Quick Start

### Prerequisites

- **Rust** 1.85+ (`rustup default stable`)
- **Bun** (for frontend, [install](https://bun.sh))
- **SQLite** 3.40+ (system package)
- **Docker** (optional, for containerized testing)

### Setup

```bash
git clone https://github.com/codecoradev/titen.git
cd titen

# Backend — run tests
cargo test

# Frontend
cd web
bun install
bun run dev
```

### Development Server

```bash
# Start API + optional MCP server
cargo run -- serve --port 7845

# Or via CLI
cargo run -- serve --port 7845 --mcp

# Frontend (separate terminal)
cd web && bun run dev
```

Set `TITEN_API_KEY` if you want authentication enabled. Otherwise, the API runs in dev mode (no auth).

## Project Structure

```
titen/
├── crates/
│   ├── titen-core/     # Domain logic, models, Store (SQLite), Threads API client
│   ├── titen-api/      # HTTP API server (Axum), auth, routes, migrations
│   ├── titen-cli/      # CLI client (`titen` binary)
│   └── titen-mcp/      # MCP server (JSON-RPC over stdio for AI agents)
├── web/                # SvelteKit frontend (Svelte 5 runes + shadcn-svelte)
├── docs/               # Documentation
└── docker-compose.yml  # Production deployment
```

### Crate Dependencies

```
titen-core  ←  titen-api  ←  titen-cli
                titen-api  ←  titen-mcp
```

`titen-core` is the foundation — no dependencies on other crates. `titen-api` depends on `titen-core`. `titen-cli` and `titen-mcp` depend on both.

## Coding Standards

### Rust

- **Edition**: 2024
- **MSRV**: 1.85
- **Formatting**: `cargo fmt --all` (enforced in CI)
- **Linting**: `cargo clippy --all-targets -D warnings` (enforced in CI)
- **Tests**: All new features must include tests. Run `cargo test --all` before pushing.

### Frontend

- **Framework**: SvelteKit + Svelte 5 runes
- **UI**: shadcn-svelte + Tailwind CSS v4
- **Package Manager**: **Bun only** — do not use npm or pnpm
- **Build**: `bun run build`

### Git Conventions

- **Branch from**: `develop` (default branch)
- **PR target**: `develop`
- **Branch naming**: `feat/...`, `fix/...`, `test/...`, `docs/...`, `chore/...`
- **Commit message**: Conventional Commits style
  ```
  feat: add video upload support
  fix: resolve timezone mismatch in scheduler
  test: add MCP tool handler tests
  docs: update API reference
  chore: bump dependencies
  ```
- **Squash merge** — all PRs are squashed on merge

### Pre-commit Hook

The repo uses a pre-commit hook that runs:

1. `cargo fmt --check` — formatting must be clean
2. `cargo clippy --all-targets -D warnings` — zero warnings allowed
3. `cora review --staged` — AI code review

**Never use `--no-verify` to skip the hook.** Fix the issues instead.

### CI Checks (GitHub Actions)

All PRs must pass these checks before merge:

| Check | Description |
|-------|-------------|
| **Check** | `cargo check --all` |
| **Format** | `cargo fmt --all --check` |
| **Clippy** | `cargo clippy --all-targets -D warnings` |
| **Test** | `cargo test --all` |
| **Build** | `cargo build --release` |
| **Frontend Security Audit** | Bun dependency audit |
| **Cargo Audit** | Rust vulnerability scan |
| **Trivy FS Scan** | Filesystem security scan |

### Linear History

`develop` maintains linear history. Use `git rebase origin/develop` if your branch falls behind.

## Testing

### Backend Tests

```bash
# All tests
cargo test --all

# Specific crate
cargo test -p titen-core
cargo test -p titen-api
cargo test -p titen-cli
cargo test -p titen-mcp
```

Tests use in-memory SQLite (`sqlite::memory:`) — no external database needed.

### Frontend Tests

```bash
cd web
bun run test
```

### Integration Testing

For manual integration testing, start the full stack:

```bash
# Terminal 1: API server
TITEN_API_KEY=test-key cargo run -- serve

# Terminal 2: Frontend
cd web && bun run dev

# Terminal 3: Test with CLI
TITEN_URL=http://localhost:7845 TITEN_API_KEY=test-key cargo run -- account list
```

## Database Migrations

Migrations are in `crates/titen-api/migrations/` as numbered SQL files:

```
001_initial.sql
002_add_reply_tracking.sql
...
011_sessions_table.sql
```

### Adding a Migration

1. Create `crates/titen-api/migrations/NNN_description.sql`
2. Register it in `crates/titen-core/src/store.rs` `migrate()` function
3. Use `CREATE TABLE IF NOT EXISTS` for idempotency
4. Test with a fresh database

## Adding MCP Tools

MCP tools are defined in `crates/titen-mcp/src/main.rs`:

1. Add tool name + JSON schema to the tool list (in `list_tools()` response)
2. Add handler in the `handle_tool()` match statement
3. Document the tool in the module-level doc comment
4. Add test coverage

## Security Considerations

- **Never log API keys, tokens, or session tokens**
- **Never commit `.env` files** — use `.env.example` for documentation
- **Encryption keys**: Generated automatically on first run, stored at `~/.codecoradev/titen/`
- **Sessions**: Persisted to SQLite, 7-day TTL, 256-bit opaque tokens
- **Production**: Server refuses to start if `TITEN_API_KEY` is unset (fail-closed)

## Reporting Issues

- **Bugs**: Open a [GitHub Issue](https://github.com/codecoradev/titen/issues) with steps to reproduce
- **Security vulnerabilities**: Email security@codecoradev.com (do NOT open a public issue)
- **Feature requests**: Open an issue with the `enhancement` label

## License

By contributing, you agree that your contributions will be licensed under the [Apache-2.0 License](LICENSE).
