-- 013_normalize_timestamps.sql
-- Mention and comment timestamps existed in mixed formats: Threads Graph API
-- `+0000` offsets, RFC3339 `+00:00`/`Z` from Titen writers, and SQLite
-- `datetime('now')` space format from schema defaults. Lexicographic
-- comparisons (trends horizon filter: fetched_at >= <RFC3339>) silently
-- excluded the space-format rows, and strict RFC3339 parsing dropped the
-- `+0000` rows. Canonicalize everything to `YYYY-MM-DDTHH:MM:SSZ`.
--
-- Guards keep this idempotent: rows already ending in `Z` or carrying a
-- `+HH:MM` offset are skipped, and any expression SQLite cannot parse is
-- left untouched (strftime returns NULL → WHERE rejects the row).

-- Threads `+0000` style first — SQLite cannot parse offsets without a colon.
UPDATE mentions SET mentioned_at = strftime('%Y-%m-%dT%H:%M:%SZ', replace(mentioned_at, '+0000', '+00:00'))
WHERE mentioned_at LIKE '%+0000'
  AND strftime('%Y-%m-%dT%H:%M:%SZ', replace(mentioned_at, '+0000', '+00:00')) IS NOT NULL;

UPDATE mentions SET mentioned_at = strftime('%Y-%m-%dT%H:%M:%SZ', mentioned_at)
WHERE mentioned_at IS NOT NULL
  AND mentioned_at NOT LIKE '%Z'
  AND mentioned_at NOT LIKE '%+_:__'
  AND strftime('%Y-%m-%dT%H:%M:%SZ', mentioned_at) IS NOT NULL;

UPDATE mentions SET fetched_at = strftime('%Y-%m-%dT%H:%M:%SZ', replace(fetched_at, '+0000', '+00:00'))
WHERE fetched_at LIKE '%+0000'
  AND strftime('%Y-%m-%dT%H:%M:%SZ', replace(fetched_at, '+0000', '+00:00')) IS NOT NULL;

UPDATE mentions SET fetched_at = strftime('%Y-%m-%dT%H:%M:%SZ', fetched_at)
WHERE fetched_at NOT LIKE '%Z'
  AND fetched_at NOT LIKE '%+_:__'
  AND strftime('%Y-%m-%dT%H:%M:%SZ', fetched_at) IS NOT NULL;

UPDATE comments SET fetched_at = strftime('%Y-%m-%dT%H:%M:%SZ', replace(fetched_at, '+0000', '+00:00'))
WHERE fetched_at LIKE '%+0000'
  AND strftime('%Y-%m-%dT%H:%M:%SZ', replace(fetched_at, '+0000', '+00:00')) IS NOT NULL;

UPDATE comments SET fetched_at = strftime('%Y-%m-%dT%H:%M:%SZ', fetched_at)
WHERE fetched_at NOT LIKE '%Z'
  AND fetched_at NOT LIKE '%+_:__'
  AND strftime('%Y-%m-%dT%H:%M:%SZ', fetched_at) IS NOT NULL;
