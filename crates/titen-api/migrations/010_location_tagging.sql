-- Add location_id column to schedules table for location tagging support.
-- NULL = no location tag (backward compatible with existing schedules).
-- Non-NULL = Threads location ID to be passed to the container creation API.
ALTER TABLE schedules ADD COLUMN location_id TEXT;
