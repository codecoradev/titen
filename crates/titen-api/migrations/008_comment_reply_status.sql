-- 008: Comment reply status workflow + sentiment priority
-- Add reply tracking columns to comments table

ALTER TABLE comments ADD COLUMN reply_status TEXT NOT NULL DEFAULT 'new';
-- values: new | needs_reply | replied | skipped

ALTER TABLE comments ADD COLUMN replied_at TEXT;
ALTER TABLE comments ADD COLUMN reply_text TEXT;
ALTER TABLE comments ADD COLUMN assigned_priority INTEGER NOT NULL DEFAULT 0;

-- Index for efficient filtering by reply_status
CREATE INDEX IF NOT EXISTS idx_comments_reply_status ON comments(reply_status);

-- Index for efficient filtering by sentiment
CREATE INDEX IF NOT EXISTS idx_comments_sentiment ON comments(sentiment);
