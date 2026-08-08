-- Issue #80: media_urls column documentation
-- media_urls is TEXT storing a JSON-encoded array of URL strings.
-- SQLite has no native JSON column type; TEXT + serde_json is the correct pattern.
-- This migration is a no-op marker — the column already exists from 001_initial.
-- No schema change needed; the fix is in Rust model type alignment.
SELECT 1;
