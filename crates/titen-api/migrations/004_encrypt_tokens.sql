-- 004_encrypt_tokens.sql
-- Migration marker: token encryption is handled in Rust (store.rs migrate()).
--
-- This file exists so the SQL migration loop processes it and creates a
-- tracking table. The actual encryption of existing plaintext data is
-- performed by Store::migrate_encrypted_fields() in Rust after all SQL
-- migrations complete, because AES-256-GCM cannot run inside SQLite.
--
-- Schema change: none (columns remain TEXT, values change format).
-- Existing plaintext values are upgraded to enc:v1:<base64> format.

CREATE TABLE IF NOT EXISTS _encryption_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
