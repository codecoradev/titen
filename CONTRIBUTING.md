# Contributing to Titen

Titen is a solo-maintained project with a strong product direction. Contributions are welcome, but **alignment matters more than volume**.

This document helps you decide *whether* and *how* to contribute in a way that's likely to get merged, so neither of us wastes time.

## How this project is run

- Titen has one active maintainer ([@ajianaz](https://github.com/ajianaz)).
- Review bandwidth is limited.
- Not every contribution can be accepted, even if it's technically correct. Alignment with project direction matters as much as code quality.
- For scope and direction, check open issues. Read them before opening anything non-trivial.

This is normal for a solo project. A "no" on a PR is not personal.

## Quick start

```bash
# Prerequisites: Rust 1.88+ (via https://rustup.rs), Bun, Git
git clone https://github.com/codecoradev/titen.git
cd titen
cargo build --workspace
cargo test --workspace
```

### Frontend (SvelteKit)

```bash
cd web
bun install
bun run dev    # dev server at localhost:5173 (proxies /api to localhost:7845)
bun run build  # production build
```

## Where to discuss

Use GitHub Issues for tracking concrete bugs and features. For design discussions or "should I work on X?", open an issue first.

## What makes a good contribution

These get merged fast:

- **Bug fixes** with clear reproduction steps and tests.
- **Docs / typos / small UX fixes**. Open a PR directly.
- **Pre-discussed features**. Alignment in an issue first.
- **Small, focused changes**. Easy to review, low risk.

If your change is small and obvious (typo, narrow bugfix, small docs change), open a PR directly. No issue required.

## Keep changes focused

**Only change what's needed to accomplish your stated goal.**

If you're fixing a bug in `store.rs`, don't also:

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
- Changes to the token management, scheduler, or Threads API client subsystems

Pull requests with significant unsolicited changes will be closed without detailed review. This isn't meant to discourage contribution. It ensures alignment before significant work goes in.

A 10-minute conversation saves a 500-line PR that doesn't fit the roadmap.

## Quality bar

Every PR is reviewed against:

- `cargo fmt --all -- --check`. Must be clean.
- `cargo clippy --workspace --all-targets -- -D warnings`. Must be clean.
- `cargo test --workspace`. Must pass.
- `bun run check` and `bun run build` (if web/ is touched). Must pass.
- [Cora review](https://github.com/codecoradev/cora-code). Run locally before pushing (`cora review --base origin/develop`).
- No new heavy dependencies without justification.
- No perf regressions in hot paths: scheduler, Threads API client, SQLite queries.

If you're not sure how to measure perf or what counts as a hot path, ask in an issue. Better to confirm than get bounced.

## Changes to core subsystems require a test

The most common way a PR breaks Titen is a **local fix with global blast radius**: the diff solves one case, reads fine, passes clippy, and silently breaks the same subsystem in other cases. Review alone does not catch these. A test does.

If your change touches behavior in any of these load-bearing paths, the PR must add or extend a test:

- **Token management (store.rs)**: SQLite writes, schema migrations, encryption/decryption
- **Threads API client (threads_client.rs)**: container creation, publishing flow, rate limit handling
- **Scheduler (scheduler.rs)**: due schedule processing, stale schedule reaping
- **API routes (routes/)**: request handling, response formatting, error handling
- **Schema migrations (migrations/)**: version upgrades, data migration

The bar for the test is real coverage of the contract, not a placeholder. Test the edge case that would actually break. If you can't see how to test it, ask in an issue before opening the PR.

UI rendering and anything the type-checker already guarantees do not need tests.

## What Titen is not

To set expectations:

- Not building a social media management dashboard with manual editing UI.
- Not building: multi-user collaboration, team management, enterprise SSO.
- Not a curated "first open-source contribution" project. Beginners are welcome but expect normal review.
- Mechanical refactors, broad style changes, drive-by rewrites are not helpful.
- AI-assisted contributions are welcome, but the PR must reflect understanding of the existing patterns. Low-effort AI-generated code that wasn't read by the author will be closed.

## Branch strategy

Titen uses a single-branch flow (since v0.2.0):

```
feature/* → develop (via PR) → tag vX.Y.Z (from develop HEAD) → release pipeline
```

- `develop` is the default and only long-lived branch. There is no `main` branch.
- Feature branches are created from `develop`.
- Never commit directly to `develop`. Always PR.
- Tags pushed from `develop` trigger the release workflow.

### Branch prefixes (kebab-case)

| Prefix | Use for |
|--------|---------|
| `feat/` | New feature |
| `fix/` | Bug fix |
| `chore/` | Refactor, tooling, config, dependencies |
| `docs/` | Docs-only changes |
| `perf/` | Performance work |
| `security/` | Security fix or hardening |
| `refactor/` | Code restructuring |
| `test/` | Test additions or changes |

Examples: `feat/scheduled-carousel`, `fix/token-refresh`, `security/encrypt-tokens-at-rest`.

Don't open PRs from your fork's `develop` or `main` branch. Work on a feature branch.

## Commits and PRs

The **PR title becomes the squash commit** for most PRs. Title must follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(scheduler): add cron-based thread publishing
fix(auth): handle expired tokens gracefully
chore(deps): bump axum to 0.8.1
security(store): encrypt tokens at rest
docs(readme): update installation instructions
```

Types: `feat`, `fix`, `chore`, `docs`, `perf`, `refactor`, `test`, `build`, `ci`, `security`.

Common scopes: `store`, `scheduler`, `server`, `cli`, `posts`, `auth`, `threads`, `media`, `analytics`, `comments`, `web`.

**Fill out the PR template.** Include: what changed, why, how you tested. The more specific, the faster the review.

**Open a draft PR early** if you want feedback mid-flight. Mark "Ready for review" when done.

### What gets merged faster

- Clear problem statement
- Small, focused diff
- Follows existing patterns (read 2 or 3 nearby files before writing yours)
- All checks pass (fmt, clippy, tests, web build)
- Manual testing notes describing the steps you took

### What gets bounced back

- Mixed-concern PRs
- Large architectural PRs without prior discussion
- New dependencies without justification
- Breaking changes without migration notes
- Incidental reformatting unrelated to the change
- AI-generated code that obviously wasn't read by the author

## Code review with Cora

[Cora](https://github.com/codecoradev/cora-code) is an AI-powered code review tool that runs automatically on every PR via CI. It produces SARIF output and posts review comments grouped by severity.

### CI (automatic)

Every PR to `develop` triggers the `Cora Review` CI job:

- Runs `cora review --base origin/develop`
- Posts results as a PR comment
- **Blocks merge** if any Error-level issues are found

### Local (recommended)

Run Cora locally **before pushing** to catch issues early:

```bash
# Review your uncommitted changes
cora review --base HEAD~1 --format text

# Review against develop
cora review --base origin/develop --format text
```

## CI checks (all must pass)

| Check | Command |
|-------|---------|
| Check | `cargo check --workspace --all-targets` |
| Format | `cargo fmt --all -- --check` |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` |
| Test | `cargo test --workspace` |
| Build | `cargo build --all-targets --release` |
| Web check | `bun run check` (if web/ exists) |
| Web build | `bun run build` (if web/ exists) |
| Web test | `bun run test` (if web/ exists) |

## Code style

- Follow existing patterns. Read 2 or 3 adjacent files before adding new ones.
- Rust: `cargo fmt` + `cargo clippy` clean. Clippy warnings are errors in CI.
- Frontend: Svelte 5 runes. Check existing components for conventions.
- Comments: only for *why*, not *what*. Code should explain itself.
- No emojis in code or commit messages.

## Architecture

Titen is a Cargo workspace with four crates, plus a SvelteKit frontend:

| Crate | Purpose | Binary |
|-------|---------|--------|
| `titen-core` | Library: models, DB store, Threads API client, sentiment | |
| `titen-api` | HTTP API server (Axum) | `titen-api` |
| `titen-cli` | CLI interface (Clap) | `titen` |
| `titen-mcp` | MCP server for AI agent integration | `titen-mcp` |

```
crates/
├── titen-core/           # Domain logic, types, error handling
│   └── src/
│       ├── lib.rs        # Module exports
│       ├── models.rs     # Data structs (Account, Post, Comment, etc.)
│       ├── store.rs      # SQLite store + migrations + encryption
│       ├── crypto.rs     # AES-256-GCM token encryption
│       ├── threads_client.rs  # Threads Graph API client
│       └── scheduler.rs  # Background scheduler
├── titen-api/            # HTTP server
│   └── src/
│       ├── server.rs     # Axum server setup + middleware
│       └── routes/       # API endpoint handlers
├── titen-cli/            # CLI binary
│   └── src/
│       └── main.rs       # Clap command definitions
└── titen-mcp/            # MCP server
    └── src/
        └── tools.rs      # MCP tool handlers

web/                      # SvelteKit frontend (adapter-node SSR)
├── src/
│   ├── routes/           # Admin pages + landing page
│   └── lib/              # Shared components + API client
```

### Key design decisions

- **SQLite-first**. Single database file, sqlx compile-time checked queries.
- **Application-layer token encryption**. AES-256-GCM on `access_token` and `app_secret` only, controlled by `TITEN_ENCRYPTION_KEY`.
- **Two-container Docker setup**. SvelteKit SSR (Bun) + Rust API. The web container proxies `/api/*` to the API container.
- **Single-branch flow**. No `main` branch since v0.2.0. Tags pushed from `develop` HEAD.
- **Scheduler runs in-process**. Started alongside the API server, not as a separate service.

## Reporting issues

- **Bugs**: Use the [Bug Report](https://github.com/codecoradev/titen/issues/new?template=bug_report.yml) template.
- **Features**: Use the [Feature Request](https://github.com/codecoradev/titen/issues/new?template=feature_request.yml) template.

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

By contributing you agree your work is licensed under [MIT](LICENSE). No CLA required.
