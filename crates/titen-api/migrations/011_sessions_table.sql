-- 011_sessions_table.sql
-- Persistent session store (replaces in-memory DashMap)

CREATE TABLE IF NOT EXISTS sessions (
    token        TEXT PRIMARY KEY,
    api_key      TEXT NOT NULL,
    expires_at   INTEGER NOT NULL,
    created_at   TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_sessions_expires ON sessions(expires_at);
