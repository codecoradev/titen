-- 002_drop_refresh_token.sql
-- Threads API uses access_token only for refresh (no separate refresh_token)
ALTER TABLE accounts DROP COLUMN refresh_token;
