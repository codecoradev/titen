-- 009_app_settings.sql
-- Centralized application settings (single-row table).
-- Sensitive fields (threads_app_secret) are encrypted at rest via AES-256-GCM.

CREATE TABLE IF NOT EXISTS app_settings (
    id                        INTEGER PRIMARY KEY DEFAULT 1,
    instance_name             TEXT NOT NULL DEFAULT '',
    auto_fetch_comments       INTEGER NOT NULL DEFAULT 1,
    comment_fetch_interval    TEXT NOT NULL DEFAULT '30',
    schedule_lookahead_hours  TEXT NOT NULL DEFAULT '24',
    threads_app_id            TEXT,
    threads_app_secret_enc    TEXT,
    updated_at                TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK (id = 1)
);

-- Seed the single row so subsequent UPSERTs always hit.
INSERT OR IGNORE INTO app_settings (id) VALUES (1);
