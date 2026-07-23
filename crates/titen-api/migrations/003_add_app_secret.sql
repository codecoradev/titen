-- 003_add_app_secret.sql
-- Store app_secret for short-lived → long-lived token exchange
ALTER TABLE accounts ADD COLUMN app_secret TEXT;
