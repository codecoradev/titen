# Contributing to Titen

Thanks for your interest in contributing! This guide covers the basics.

## Prerequisites

- **Rust** 1.88+ — install via [rustup](https://rustup.rs/)
- **Git**

## Build

```bash
git clone https://github.com/codecoradev/titen.git
cd titen
cargo build --workspace
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

## License

By contributing, you agree that your contributions will be licensed under the project license.
