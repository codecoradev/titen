-- 014: schedulable thread replies (#232)
--
-- Adds reply_to_id to schedules. When set, the scheduler publishes the
-- schedule as a reply to that Threads post (media_type must be TEXT)
-- instead of creating a root-level post.
--
-- The value is the raw Threads post/media ID (numeric Graph API ID, e.g.
-- "12345678901234567"), the same ID format accepted by the existing
-- real-time reply endpoint (POST /api/posts/{id}/reply).
--
-- Idempotency: applied via the same duplicate-column-tolerant pattern as 005.

ALTER TABLE schedules ADD COLUMN reply_to_id TEXT;
