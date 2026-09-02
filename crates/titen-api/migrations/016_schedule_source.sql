-- 016: schedule source identifier (#242 — agent ingest)
--
-- Free-form origin tag for schedules, e.g. "cmo-agent" or "ci-pipeline".
-- Set at creation time (regular creates leave it NULL); the web UI can badge
-- calendar items with their origin. Immutable after creation.
--
-- Idempotency: duplicate-column-tolerant like 005/014.

ALTER TABLE schedules ADD COLUMN source TEXT;
