-- 012_post_permalink.sql
-- Store the Threads shortcode permalink (e.g. https://www.threads.com/@user/post/Db6gSYlE15d)
-- returned by the Graph API publish response. Numeric media IDs are NOT valid
-- in web URLs — the "View on Threads" link was previously built from the
-- numeric ID and led to a 404.

ALTER TABLE posts ADD COLUMN permalink TEXT;
