-- Mentions table — persist posts where the user is mentioned
-- Previously fetch_mentions hit Threads API live with no DB persistence (data loss on each call)
-- Issue #78
CREATE TABLE IF NOT EXISTS mentions (
    id                 TEXT PRIMARY KEY,
    account_id         TEXT NOT NULL REFERENCES accounts(id),
    threads_mention_id TEXT,
    author_username    TEXT,
    author_user_id     TEXT,
    text               TEXT,
    media_type         TEXT,
    permalink          TEXT,
    mentioned_at       TEXT,
    fetched_at         TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_mentions_account ON mentions(account_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_mentions_threads_id ON mentions(account_id, threads_mention_id);
CREATE INDEX IF NOT EXISTS idx_mentions_fetched ON mentions(fetched_at);
