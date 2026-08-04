# Contributing to Titen

Thanks for your interest in contributing! This guide covers the basics.

## Prerequisites

- **Rust** 1.88+ — install via [rustup](https://rustup.rs/)
- **Node.js** 22+ and **Bun** (for frontend development)
- **Git**

## Build

### Backend (Rust)

```bash
git clone https://github.com/codecoradev/titen.git
cd titen
cargo build --workspace
```

### Frontend (SvelteKit)

```bash
cd web
bun install
bun run dev    # dev server at localhost:5173 (proxies /api → localhost:7845)
bun run build  # production build to web/build/
```

## Test

```bash
# Run all tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p titen-core
cargo test -p titen-api
```

## Code Style

```bash
# Format check — must pass
cargo fmt --all -- --check

# Lint — must pass with no warnings
cargo clippy --workspace --all-targets -- -D warnings
```

Run `cargo fmt` before committing. Clippy warnings are treated as errors in CI.

## Submitting a PR

1. **Fork** the repository
2. **Create a branch** from `develop` — use descriptive names like `fix/auth-crash` or `feat/scheduling`
3. **Make your changes** — keep PRs focused and small
4. **Add tests** for new functionality
5. **Run checks locally** — `cargo fmt`, `cargo clippy`, `cargo test` all green
6. **Open a Pull Request** against the `develop` branch

### CI Checks

Every PR runs these checks (all must pass before merge):

| Check | Description |
|-------|-------------|
| Check | `cargo check --workspace --all-targets` |
| Format | `cargo fmt --all -- --check` |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` |
| Test | `cargo test --workspace` |
| Build | `cargo build --release` |

## Commit Messages

Use clear, descriptive commit messages:

```
feat(scheduling): add cron-based thread publishing
fix(auth): handle expired tokens gracefully
docs: update README with deployment guide
refactor: extract error handling into shared module
```

Prefix with type: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`.

## Architecture

Titen is a Cargo workspace with four crates:

```
crates/
├── titen-core/     # Domain logic, types, error handling
├── titen-api/       # HTTP server (Axum + SQLx + SQLite)
├── titen-cli/       # CLI binary
└── titen-mcp/       # MCP server for AI tool integration
```

## Reporting Issues

- **Bugs:** Use the [Bug Report](https://github.com/codecoradev/titen/issues/new?template=bug_report.md) template
- **Features:** Use the [Feature Request](https://github.com/codecoradev/titen/issues/new?template=feature_request.md) template

## Security

**Do not open public issues for security vulnerabilities.**

If you discover a security issue, please email **security@codecora.dev** with a detailed description and reproduction steps. We will acknowledge receipt within 48 hours and aim to publish a fix within 7 days.

## Branch Strategy

- `main` — stable, tagged releases only. Never commit directly.
- `develop` — integration branch. PRs merge here first.
- Feature branches — `feat/`, `fix/`, `docs/`, `refactor/` prefixes.

All PRs target `develop`. Releases merge `develop` → `main` + tag.

## License

By contributing, you agree that your contributions will be licensed under the project license.
