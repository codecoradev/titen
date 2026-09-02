-- 015: OAuth state tokens (#237 — CSRF protection)
--
-- One-time state tokens for the Threads OAuth flow. Each token is bound to
-- the API key / session that requested it and expires after 10 minutes.
-- The exchange endpoint atomically deletes a token on successful validation,
-- making replay impossible.
--
-- Idempotency: applied via the same duplicate-column-tolerant pattern as
-- 005/014 for the table-exists case.

CREATE TABLE IF NOT EXISTS oauth_states (
    token TEXT PRIMARY KEY,
    bound_key TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_oauth_states_expires ON oauth_states (expires_at);
