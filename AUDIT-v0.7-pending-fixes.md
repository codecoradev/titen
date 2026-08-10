# Titen v0.7 Audit — Pending Fixes

## Findings dari Review Comments (belum di-fix)

### 🔴 Critical
1. **PR #154 — SQL injection di `VACUUM INTO '{dest}'`** (`crates/titen-cli/src/main.rs:59`)
   - `dest` di-interpolate langsung ke SQL string. Path dengan single quote (e.g. `foo'.db`) bisa break SQL atau allow arbitrary SQL execution.
   - **Fix**: Escape single quotes (`dest.replace("'", "''")`) atau validasi path tidak mengandung `'`.

### 🟡 Medium
2. **PR #154 — Restore overwrites live DB without coordination** (`main.rs:71`)
   - `restore_database` copies over DB file while server may still hold connections.
   - **Fix**: Add warning "ensure server is stopped before restore" in CLI output.

3. **PR #154 — `.pre-restore` fixed name overwritten on re-run** (`main.rs:84`)
   - Multiple restore runs destroy previous safety-net backup.
   - **Fix**: Add timestamp to pre-restore filename.

4. **PR #153 — `/metrics` exposes operational data without auth** (`server.rs:132`)
   - Account/post/session counts + encryption status publicly accessible.
   - **Fix**: Add `TITEN_PUBLIC_METRICS=true` env toggle (default: false, requires auth).

5. **PR #153 — Auth bypass path matching bypassed** (`server.rs:184`)
   - Exact match `path == "/health"` can be bypassed with trailing slash, `./`, URL encoding.
   - **Fix**: Use `path.trim_end_matches('/').starts_with(...)` or normalize path.

6. **PR #152 — Serve defaults to `0.0.0.0`** (`lib.rs:22`)
   - Binds to all interfaces by default. Both Cora + GHAS flagged this.
   - **Fix**: Default to `127.0.0.1`, user can pass `--host 0.0.0.0` for production.

7. **PR #150 — CLA requirement contradicts original "No CLA required"** (`CONTRIBUTING.md:215`)
   - New CONTRIBUTING.md adds CLA requirement, but original explicitly says "No CLA required."
   - **Fix**: Align — remove CLA requirement from CONTRIBUTING.md, or clarify CLA is for codecoradev org.

### 🟢 Low
8. **PR #154 — Error match no wildcard arm** (`error.rs:38`)
   - `_ =>` arm missing. If `TitenError` gains new variants, won't compile.
   - **Fix**: Add `_ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")`.

9. **PR #150 — Rust version doc inconsistency** (`CONTRIBUTING.md:8`)
   - Original says 1.88+, new says 1.85. Actual `rust-version` in Cargo.toml = 1.85.
   - **Fix**: Standardize to match Cargo.toml (1.85).

10. **PR #153 — `datetime('now')` SQLite-specific** (`store.rs:82`)
    - **FALSE POSITIVE** — Titen is SQLite-only by design. No action needed.

## Summary

| Severity | Count | Action |
|----------|-------|--------|
| Critical | 1 | Fix before v0.7.0 release |
| Medium | 5 | Fix in v0.7.1 |
| Low | 3 (1 false positive) | Fix in v0.8.0 |

## Recommended Fix Order (next session)
1. SQL injection fix (critical)
2. `0.0.0.0` → `127.0.0.1` default
3. Auth bypass path normalization
4. Pre-restore timestamp
5. Error wildcard arm
6. Restore warning message
7. Metrics auth toggle
8. CONTRIBUTING.md fixes
