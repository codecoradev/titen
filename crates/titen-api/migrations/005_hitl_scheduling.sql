-- 005: HITL scheduling — require human approval before auto-publishing
--
-- New schedule lifecycle:
--   draft → pending (approved) → processing → published/failed
--   draft → rejected
--
-- Existing schedules with status='pending' are migrated to 'pending' (already approved).
-- Default for new schedules becomes 'draft' (requires approval before scheduler picks up).

-- Add approval tracking columns
ALTER TABLE schedules ADD COLUMN approved_by TEXT;
ALTER TABLE schedules ADD COLUMN approved_at TEXT;

-- Update default status for future inserts to 'draft'
-- SQLite doesn't support ALTER COLUMN DEFAULT, so we handle this at the application layer.
-- The store.rs create_schedule will explicitly set status='draft' for new schedules.
