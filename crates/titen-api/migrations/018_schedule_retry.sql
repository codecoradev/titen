-- 018 — Transient-error retry for scheduled publishes (#257)
--
-- Two retry-planning columns on schedules:
--   attempt_count  — how many publish attempts have been made (0 = never tried)
--   next_due_at    — when the schedule becomes eligible again. NULL = use
--                    scheduled_at (original behavior). On transient failure the
--                    scheduler backfills this with a +10min delay and resets the
--                    row to 'pending'; on permanent failure it stays NULL.
ALTER TABLE schedules ADD COLUMN attempt_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE schedules ADD COLUMN next_due_at TEXT;
