# Contributing to Titen

Titen is a solo-maintained project with a strong product direction. Contributions are welcome, but **alignment matters more than volume**.

This document helps you decide *whether* and *how* to contribute in a way that's likely to get merged, so neither of us wastes time.

## How this project is run

- Titen has one active maintainer ([@ajianaz](https://github.com/ajianaz)).
- Review bandwidth is limited.
- Not every contribution can be accepted, even if it's technically correct. Alignment with project direction matters as much as code quality.
- For scope and direction, check open issues and [ROADMAP.md](ROADMAP.md) if available. Read them before opening anything non-trivial.

This is normal for a solo project. A "no" on a PR is not personal.

## Quick start

```bash
# Prerequisites: Rust 1.85+ (via https://rustup.rs), Bun (https://bun.sh), SQLite 3.40+
git clone https://github.com/codecoradev/titen.git
cd titen

# Backend
cargo build --workspace
cargo test --workspace

# Frontend
cd web
bun install
bun run dev
```

### Development server

```bash
# Start API + optional MCP server
cargo run -- serve --port 7845

# Or with MCP enabled
cargo run -- serve --port 7845 --mcp

# Frontend (separate terminal)
cd web && bun run dev
```

Set `TITEN_API_KEY` if you want authentication enabled. Otherwise, the API runs in dev mode (no auth).

## Where to discuss

Use GitHub Issues for tracking concrete bugs and features. For design discussions or "should I work on X?", open an issue first.

## What makes a good contribution

These get merged fast:

- **Bug fixes** with clear reproduction steps and tests.
- **Docs / typos / small UX fixes** — open a PR directly.
- **Pre-discussed features** — alignment in an issue first.
- **Small, focused changes** — easy to review, low risk.

If your change is small and obvious (typo, narrow bugfix, small docs change), open a PR directly. No issue required.

## Keep changes focused

**Only change what's needed to accomplish your stated goal.**

If you're fixing a bug in `scheduler.rs`, don't also:

- Reformat other files
- Clean up unrelated code
- Fix lint issues in files you didn't need to touch
- Combine multiple unrelated fixes in one PR

**One PR = one logical change.** Multi-concern PRs will be asked to split.

## Discuss first (required for larger changes)

For anything beyond a small fix, **discussion is required before opening a PR**. This includes:

- New features
- API changes or new endpoints
- Refactors or "cleanup" work
- Performance rewrites
- Architectural changes
- Anything touching many files or subsystems
- Changes to the token management, Threads API client, scheduler, or schema migration subsystems

Pull requests with significant unsolicited changes will be closed without detailed review. This isn't meant to discourage contribution. It ensures alignment before significant work goes in.

A 10-minute conversation saves a 500-line PR that doesn't fit the roadmap.

## Quality bar

Every PR is reviewed against:

- `cargo fmt --all -- --check` — must be clean
- `cargo clippy --workspace --all-targets -- -D warnings` — must be clean
- `cargo test --workspace` — must pass
- [Cora review](https://github.com/codecoradev/cora-cli) — run locally before pushing (`cora review --base origin/develop`)
- `cargo build --release --workspace` — must compile
- No new heavy dependencies without justification
- No perf regressions in hot paths: token refresh, Threads API calls, scheduler execution, API response latency

If you're not sure how to measure perf or what counts as a hot path, ask in an issue. Better to confirm than get bounced.

### Frontend quality bar

If you touched `web/`:

- `bun run check` passes
- `bun run build` passes
- Uses **shadcn-svelte** components only (no raw Tailwind components)
- **Bun only** — do not use npm or pnpm

## Changes to core subsystems require a test

The most common way a PR breaks Titen is a **local fix with global blast radius**: the diff solves one case, reads fine, passes clippy, and silently breaks the same subsystem in other cases. Review alone does not catch these. A test does.

If your change touches behavior in any of these load-bearing paths, the PR must add or extend a test:

- **Token management (store.rs)**: encryption, refresh logic, expiry checks
- **Threads API client**: request/response handling, rate limiting, error handling
- **Scheduler**: cron execution, post scheduling, timezone handling
- **Schema migration**: version upgrades, data migration
- **API server (titen-api)**: endpoint registration, auth middleware, request/response handling
- **CLI commands (titen-cli)**: argument parsing, output formatting
- **MCP server (titen-mcp)**: tool registration, handler dispatch, JSON-RPC protocol

The bar for the test is real coverage of the contract, not a placeholder. Test the edge case that would actually break. If you can't see how to test it, ask in an issue before opening the PR.

## What Titen is not

To set expectations:

- Not trying to be a full social media management platform (Hootsuite, Buffer).
- Not building: team collaboration, enterprise SSO, multi-platform publishing beyond Threads.
- Not a curated "first open-source contribution" project. Beginners are welcome but expect normal review.
- Mechanical refactors, broad style changes, drive-by rewrites are not helpful.
- AI-assisted contributions are welcome, but the PR must reflect understanding of the existing patterns. Low-effort AI-generated code that wasn't read by the author will be closed.

## Branches

Branch off `develop`. Use these prefixes (kebab-case):

| Prefix        | Use for                                  |
| ------------- | ---------------------------------------- |
| `feat/`       | New feature                              |
| `fix/`        | Bug fix                                  |
| `chore/`      | Refactor, tooling, config, dependencies  |
| `docs/`       | Docs-only changes                        |
| `perf/`       | Performance work                         |
| `security/`   | Security fix or hardening                |
| `refactor/`   | Code restructuring                       |
| `test/`       | Test additions/changes                   |

Examples: `feat/token-autorefresh`, `fix/scheduler-timezone`, `security/encrypt-tokens`.

Don't open PRs from your fork's `develop` or `main` branch. Work on a feature branch.

## Commits & PRs

The **PR title becomes the squash commit** for most PRs. Title must follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(scheduler): add cron-based publishing
fix(token): handle expiry without panic
chore(deps): bump axum to 0.8
security(api): sanitize error responses
docs(readme): update installation instructions
```

Types: `feat`, `fix`, `chore`, `docs`, `perf`, `refactor`, `test`, `build`, `ci`, `security`.

Common scopes: `token`, `scheduler`, `api`, `cli`, `mcp`, `store`, `threads`, `auth`, `web`.

**Fill out the PR template.** Include: what changed, why, how you tested. The more specific, the faster the review.

**Open a draft PR early** if you want feedback mid-flight. Mark "Ready for review" when done.

### What gets merged faster

- Clear problem statement
- Small, focused diff
- Follows existing patterns (read 2–3 nearby files before writing yours)
- All checks pass (fmt, clippy, tests, Cora)
- Manual testing notes describing the steps you took

### What gets bounced back

- Mixed-concern PRs
- Large architectural PRs without prior discussion
- New dependencies without justification
- Breaking changes without migration notes
- Incidental reformatting unrelated to the change
- AI-generated code that obviously wasn't read by the author

## Code Review with Cora CLI

[Cora](https://github.com/codecoradev/cora-cli) is an AI-powered code review tool that runs automatically on every PR via CI. It uses SARIF output and posts review comments directly on the PR.

### CI (Automatic)

Every PR to `develop` triggers the CodeCora review CI job:

- Runs `cora review --base origin/develop --format sarif --severity major`
- Posts results as a PR comment (grouped by severity)
- **Blocks merge** if any Error-level issues are found

### Local (Recommended)

Run Cora locally **before pushing** to catch issues early:

```bash
# Review your uncommitted changes
cora review --base HEAD~1 --format text

# Review against develop
cora review --base origin/develop --format text
```

## Pre-commit Hook

The repo uses a pre-commit hook that runs:

1. `cargo fmt --check` — formatting must be clean
2. `cargo clippy --all-targets -D warnings` — zero warnings allowed
3. `cora review --staged` — AI code review

**Never use `--no-verify` to skip the hook.** Fix the issues instead.

## Architecture

Titen is a Cargo workspace with a SvelteKit frontend:

| Crate | Purpose |
|-------|---------|
| `titen-core` | Domain logic, models, Store (SQLite), Threads API client |
| `titen-api` | HTTP API server (Axum), auth, routes, migrations |
| `titen-cli` | CLI client (`titen` binary) |
| `titen-mcp` | MCP server (JSON-RPC over stdio for AI agents) |

```
crates/
├── titen-core/     # Foundation — no deps on other crates
├── titen-api/      # HTTP API — depends on titen-core
├── titen-cli/      # CLI binary — depends on titen-api
└── titen-mcp/      # MCP server — depends on titen-api

web/                # SvelteKit frontend (Svelte 5 runes + shadcn-svelte)
```

Crate dependency flow: `titen-core ← titen-api ← {titen-cli, titen-mcp}`

### Key Design Decisions

- **SQLite-first** — single-file database, no external DB server needed
- **AES-256-GCM token encryption** — `access_token` and `app_secret` encrypted at rest
- **Fail-closed production** — server refuses to start if `TITEN_API_KEY` is unset
- **Linear history** — `develop` uses rebase + squash merge, no merge commits
- **Edition 2024** — MSRV 1.85

### Database Migrations

Migrations are in `crates/titen-api/migrations/` as numbered SQL files. To add one:

1. Create `crates/titen-api/migrations/NNN_description.sql`
2. Register it in `crates/titen-core/src/store.rs` `migrate()` function
3. Use `CREATE TABLE IF NOT EXISTS` for idempotency
4. Test with a fresh database

### Adding MCP Tools

MCP tools are defined in `crates/titen-mcp/src/main.rs`:

1. Add tool name + JSON schema to the tool list (in `list_tools()` response)
2. Add handler in the `handle_tool()` match statement
3. Document the tool in the module-level doc comment
4. Add test coverage

## Reporting Issues

- **Bugs:** Use the [Bug Report](https://github.com/codecoradev/titen/issues/new?template=bug_report.yml) template
- **Features:** Use the [Feature Request](https://github.com/codecoradev/titen/issues/new?template=feature_request.yml) template

## FAQ

**Q: Should I ask before fixing a typo or obvious bug?**
A: No, open a PR directly.

**Q: I have an idea for a new feature.**
A: Open a GitHub issue. Don't open a PR without prior discussion.

**Q: My PR was closed without detailed feedback.**
A: Usually means it didn't align with project direction, or scope was too large to review responsibly. This is normal for a solo project.

**Q: Can I work on an open issue?**
A: Comment first to confirm it's still relevant. For anything non-trivial, discuss approach before implementing.

**Q: My PR conflicts after develop moved. Should I rebase?**
A: If the change is still relevant and reasonably small, yes. Large stale PRs may be closed with an offer to reopen after rebase.

## Security issues

Don't file them as public issues. See [SECURITY.md](SECURITY.md).

## Code of Conduct

See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## License

By contributing you agree your work is licensed under [Apache-2.0](LICENSE). No CLA required.
