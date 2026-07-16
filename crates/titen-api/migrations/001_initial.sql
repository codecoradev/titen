-- 001_initial.sql
-- titen schema v1

-- Accounts (multi-account token management)
CREATE TABLE IF NOT EXISTS accounts (
    id            TEXT PRIMARY KEY,
    username      TEXT NOT NULL UNIQUE,
    user_id       TEXT NOT NULL,
    access_token  TEXT NOT NULL,
    refresh_token TEXT,
    expires_at    TEXT NOT NULL,
    app_id        TEXT,
    is_active     INTEGER NOT NULL DEFAULT 1,
    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at    TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_accounts_username ON accounts(username);
CREATE INDEX idx_accounts_active ON accounts(is_active);

-- Posts (published posts tracking)
CREATE TABLE IF NOT EXISTS posts (
    id               TEXT PRIMARY KEY,
    threads_post_id  TEXT UNIQUE,
    account_id       TEXT NOT NULL REFERENCES accounts(id),
    media_type       TEXT NOT NULL DEFAULT 'TEXT',
    caption          TEXT,
    text_attachment   TEXT,
    carousel_children TEXT,
    status           TEXT NOT NULL DEFAULT 'draft',
    scheduled_id     TEXT REFERENCES schedules(id),
    published_at     TEXT,
    insights_json    TEXT,
    created_at       TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at       TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_posts_account ON posts(account_id);
CREATE INDEX idx_posts_status ON posts(status);
CREATE INDEX idx_posts_scheduled ON posts(scheduled_id);

-- Schedules (pending/scheduled posts)
CREATE TABLE IF NOT EXISTS schedules (
    id              TEXT PRIMARY KEY,
    account_id      TEXT NOT NULL REFERENCES accounts(id),
    media_type      TEXT NOT NULL DEFAULT 'TEXT',
    caption          TEXT,
    text_attachment   TEXT,
    media_urls      TEXT,
    scheduled_at    TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'pending',
    published_at    TEXT,
    result_post_id  TEXT,
    result_json     TEXT,
    error            TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at       TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_schedules_account ON schedules(account_id);
CREATE INDEX idx_schedules_status ON schedules(status);
CREATE INDEX idx_schedules_time ON schedules(scheduled_at);

-- Comments (fetched from owned posts)
CREATE TABLE IF NOT EXISTS comments (
    id                 TEXT PRIMARY KEY,
    post_id            TEXT NOT NULL REFERENCES posts(id),
    threads_comment_id TEXT,
    author_username    TEXT,
    author_user_id     TEXT,
    text               TEXT NOT NULL,
    sentiment          TEXT,
    sentiment_score    REAL,
    fetched_at         TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_comments_post ON comments(post_id);
CREATE INDEX idx_comments_sentiment ON comments(sentiment);

-- Analytics snapshots (time-series per post)
CREATE TABLE IF NOT EXISTS analytics_snap (
    id           TEXT PRIMARY KEY,
    post_id      TEXT NOT NULL REFERENCES posts(id),
    likes        INTEGER NOT NULL DEFAULT 0,
    replies      INTEGER NOT NULL DEFAULT 0,
    reposts      INTEGER NOT NULL DEFAULT 0,
    views        INTEGER NOT NULL DEFAULT 0,
    quotes       INTEGER NOT NULL DEFAULT 0,
    snapshot_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_analytics_post ON analytics_snap(post_id);
CREATE INDEX idx_analytics_time ON analytics_snap(snapshot_at);

-- Media assets (S3-managed files)
CREATE TABLE IF NOT EXISTS media_assets (
    id           TEXT PRIMARY KEY,
    filename     TEXT NOT NULL,
    content_type TEXT NOT NULL,
    size_bytes   INTEGER NOT NULL,
    s3_key       TEXT NOT NULL,
    s3_url       TEXT,
    uploaded_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_media_filename ON media_assets(filename);

-- Rate limit tracking (sliding window)
CREATE TABLE IF NOT EXISTS rate_tracking (
    id           TEXT PRIMARY KEY,
    account_id   TEXT NOT NULL REFERENCES accounts(id),
    action_type  TEXT NOT NULL,
    timestamp    TEXT NOT NULL DEFAULT (datetime('now')),
    count        INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_rate_account_action ON rate_tracking(account_id, action_type);
CREATE INDEX idx_rate_timestamp ON rate_tracking(timestamp);
