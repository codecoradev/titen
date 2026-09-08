-- 017 — thread bundles (#thread-bundle Phase 2)
-- A bundle is N schedule rows sharing bundle_id. seq orders execution;
-- total is the item count (integrity check). Root post is seq 0.
-- Rows keep their own status/result so partial failures are visible per item.
ALTER TABLE schedules ADD COLUMN bundle_id TEXT;
ALTER TABLE schedules ADD COLUMN bundle_seq INTEGER;
ALTER TABLE schedules ADD COLUMN bundle_total INTEGER;
CREATE INDEX IF NOT EXISTS idx_schedules_bundle ON schedules(bundle_id);
