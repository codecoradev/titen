use crate::crypto::Cipher;
use crate::error::{Result, TitenError};
use crate::models::*;
use sqlx::SqlitePool;

/// Split SQL text into executable statements.
///
/// Strips `-- comment` lines first, then splits on `;`. This prevents
/// semicolons inside SQL comments (e.g. "TEXT; serde_json") from breaking
/// statement splitting.
fn split_sql_statements(sql: &str) -> Vec<String> {
    let stripped: String = sql
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<Vec<&str>>()
        .join("\n");
    stripped
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Main store — SQLite database access for all titen entities.
///
/// Holds an optional [`Cipher`] for encrypting sensitive fields (`access_token`,
/// `app_secret`) at rest. When `cipher` is `None`, the store operates in
/// plaintext mode (development only — production must provide a key via
/// `TITEN_ENCRYPTION_KEY`).
pub struct Store {
    pool: SqlitePool,
    cipher: Option<Cipher>,
}

impl Store {
    /// Create a store with encryption enabled.
    ///
    /// The cipher is loaded from `TITEN_ENCRYPTION_KEY` env var.
    /// If the env var is absent, the store runs in plaintext mode (dev).
    ///
    /// Set `TITEN_REQUIRE_ENCRYPTION=true` to panic when the key is missing
    /// or invalid. This prevents accidental plaintext mode in production.
    pub fn new(pool: SqlitePool) -> Self {
        let require_encryption =
            std::env::var("TITEN_REQUIRE_ENCRYPTION").is_ok_and(|v| v == "true");

        let cipher = Cipher::from_env().unwrap_or_else(|e| {
            if require_encryption {
                panic!("TITEN_REQUIRE_ENCRYPTION is set but TITEN_ENCRYPTION_KEY is invalid: {e}");
            }
            tracing::warn!("Failed to load encryption key, running in plaintext mode: {e}");
            None
        });
        tracing::info!(
            "Store initialized with {} mode",
            if cipher.is_some() {
                "encrypted"
            } else {
                "plaintext"
            }
        );
        Self { pool, cipher }
    }

    /// Create a store with an explicit cipher (for testing).
    pub fn with_cipher(pool: SqlitePool, cipher: Option<Cipher>) -> Self {
        Self { pool, cipher }
    }

    /// Returns true if encryption is active.
    pub fn is_encrypted(&self) -> bool {
        self.cipher.is_some()
    }

    /// Encrypt a sensitive field before writing to DB.
    /// If no cipher is configured, returns the plaintext as-is.
    fn encrypt_field(&self, value: &str) -> Result<String> {
        match &self.cipher {
            Some(c) => c.encrypt(value),
            None => Ok(value.to_string()),
        }
    }

    /// Decrypt a sensitive field after reading from DB.
    /// Handles both encrypted (`enc:v1:...`) and plaintext values (backward compat).
    fn decrypt_field(&self, value: &str) -> Result<String> {
        match &self.cipher {
            Some(c) => c.decrypt(value),
            None => Ok(value.to_string()),
        }
    }

    /// Decrypt the `access_token` and `app_secret` fields of an account in-place.
    fn decrypt_account_fields(&self, account: &mut Account) -> Result<()> {
        account.access_token = self.decrypt_field(&account.access_token)?;
        if let Some(ref mut secret) = account.app_secret {
            if !secret.is_empty() {
                *secret = self.decrypt_field(secret)?;
            }
        }
        Ok(())
    }

    /// Run migrations from embedded SQL
    pub async fn migrate(&self) -> Result<()> {
        // 001 — initial schema
        for stmt in split_sql_statements(include_str!("../../titen-api/migrations/001_initial.sql"))
        {
            sqlx::query(&stmt).execute(&self.pool).await?;
        }

        // 002 — drop refresh_token column (safe for fresh installs: ignore "no such column")
        for stmt in split_sql_statements(include_str!(
            "../../titen-api/migrations/002_drop_refresh_token.sql"
        )) {
            let result = sqlx::query(&stmt).execute(&self.pool).await;
            if let Err(e) = result {
                let msg = e.to_string();
                if !msg.contains("no such column") {
                    return Err(TitenError::DatabaseError(msg));
                }
            }
        }

        // 003 — add app_secret column (safe for fresh installs: ignore "duplicate column")
        for stmt in split_sql_statements(include_str!(
            "../../titen-api/migrations/003_add_app_secret.sql"
        )) {
            let result = sqlx::query(&stmt).execute(&self.pool).await;
            if let Err(e) = result {
                let msg = e.to_string();
                if !msg.contains("duplicate column") {
                    return Err(TitenError::DatabaseError(msg));
                }
            }
        }

        // 004 — encryption metadata table
        for stmt in split_sql_statements(include_str!(
            "../../titen-api/migrations/004_encrypt_tokens.sql"
        )) {
            sqlx::query(&stmt).execute(&self.pool).await?;
        }

        // 004 (Rust side) — encrypt existing plaintext tokens if cipher is available
        self.migrate_encrypted_fields().await?;

        // 005 — HITL scheduling: add approved_by, approved_at columns
        for stmt in split_sql_statements(include_str!(
            "../../titen-api/migrations/005_hitl_scheduling.sql"
        )) {
            let result = sqlx::query(&stmt).execute(&self.pool).await;
            if let Err(e) = result {
                let msg = e.to_string();
                // SQLite error: "duplicate column" means already migrated (fresh install)
                if !msg.contains("duplicate column") {
                    return Err(TitenError::DatabaseError(msg));
                }
            }
        }

        // 006 — mentions table
        for stmt in split_sql_statements(include_str!(
            "../../titen-api/migrations/006_mentions_table.sql"
        )) {
            let result = sqlx::query(&stmt).execute(&self.pool).await;
            if let Err(e) = result {
                let msg = e.to_string();
                if !msg.contains("already exists") {
                    return Err(TitenError::DatabaseError(msg));
                }
            }
        }

        // 007 — media_urls documentation (no-op marker: SELECT 1)
        for stmt in split_sql_statements(include_str!(
            "../../titen-api/migrations/007_media_urls_doc.sql"
        )) {
            let result = sqlx::query(&stmt).execute(&self.pool).await;
            if let Err(e) = result {
                let msg = e.to_string();
                if !msg.contains("already exists") {
                    return Err(TitenError::DatabaseError(msg));
                }
            }
        }

        // 008 — comment reply status workflow (reply_status, replied_at, reply_text, assigned_priority)
        for stmt in split_sql_statements(include_str!(
            "../../titen-api/migrations/008_comment_reply_status.sql"
        )) {
            let result = sqlx::query(&stmt).execute(&self.pool).await;
            if let Err(e) = result {
                let msg = e.to_string();
                if !msg.contains("duplicate column") {
                    return Err(TitenError::DatabaseError(msg));
                }
            }
        }

        // 009 — app_settings table (centralized config, encrypted secrets)
        for stmt in split_sql_statements(include_str!(
            "../../titen-api/migrations/009_app_settings.sql"
        )) {
            let result = sqlx::query(&stmt).execute(&self.pool).await;
            if let Err(e) = result {
                let msg = e.to_string();
                if !msg.contains("already exists") {
                    return Err(TitenError::DatabaseError(msg));
                }
            }
        }

        // 010 — add location_id column to schedules (location tagging support)
        for stmt in split_sql_statements(include_str!(
            "../../titen-api/migrations/010_location_tagging.sql"
        )) {
            let result = sqlx::query(&stmt).execute(&self.pool).await;
            if let Err(e) = result {
                let msg = e.to_string();
                if !msg.contains("duplicate column") {
                    return Err(TitenError::DatabaseError(msg));
                }
            }
        }

        Ok(())
    }

    /// Encrypt any plaintext `access_token` and `app_secret` values in the DB.
    ///
    /// This runs after SQL migrations on every startup. It is idempotent:
    /// - If already encrypted (`enc:v1:` prefix), skips
    /// - If cipher is not configured (dev mode), skips entirely
    /// - If no accounts exist, no-op
    ///
    /// On first run with `TITEN_ENCRYPTION_KEY` set, this converts all
    /// existing plaintext tokens to encrypted form.
    async fn migrate_encrypted_fields(&self) -> Result<()> {
        let cipher = match &self.cipher {
            Some(c) => c,
            None => return Ok(()), // Dev mode — no encryption
        };

        // Check if we already migrated (idempotency via metadata table)
        let already: Option<(String,)> =
            sqlx::query_as("SELECT value FROM _encryption_meta WHERE key = 'tokens_encrypted_v1'")
                .fetch_optional(&self.pool)
                .await?;

        if already.is_some() {
            return Ok(()); // Already migrated
        }

        // Fetch all accounts with raw DB values (NOT decrypted)
        let rows: Vec<(String, String, Option<String>)> =
            sqlx::query_as("SELECT id, access_token, app_secret FROM accounts")
                .fetch_all(&self.pool)
                .await?;

        if rows.is_empty() {
            // No accounts — mark as done
            sqlx::query("INSERT OR IGNORE INTO _encryption_meta (key, value) VALUES ('tokens_encrypted_v1', 'true')")
                .execute(&self.pool)
                .await?;
            return Ok(());
        }

        // Wrap migration in a transaction so partial failure doesn't leave mixed state
        let mut tx = self.pool.begin().await?;
        let mut migrated = 0;

        for (id, access_token, app_secret) in rows {
            let mut row_changed = false;

            // Encrypt access_token if not already encrypted
            if !crate::crypto::is_encrypted(&access_token) {
                let enc_token = cipher.encrypt(&access_token)?;
                sqlx::query("UPDATE accounts SET access_token = ? WHERE id = ?")
                    .bind(&enc_token)
                    .bind(&id)
                    .execute(&mut *tx)
                    .await?;
                row_changed = true;
            }

            // Encrypt app_secret if present and not already encrypted
            if let Some(secret) = app_secret {
                if !secret.is_empty() && !crate::crypto::is_encrypted(&secret) {
                    let enc_secret = cipher.encrypt(&secret)?;
                    sqlx::query("UPDATE accounts SET app_secret = ? WHERE id = ?")
                        .bind(&enc_secret)
                        .bind(&id)
                        .execute(&mut *tx)
                        .await?;
                    row_changed = true;
                }
            }

            if row_changed {
                migrated += 1;
            }
        }

        // Mark migration complete within the same transaction
        sqlx::query("INSERT OR IGNORE INTO _encryption_meta (key, value) VALUES ('tokens_encrypted_v1', 'true')")
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        if migrated > 0 {
            tracing::info!("Encrypted {migrated} account token(s) at rest");
        }

        Ok(())
    }

    // ─── Accounts ───────────────────────────────────────────

    pub async fn list_accounts(&self) -> Result<Vec<Account>> {
        let mut rows =
            sqlx::query_as::<_, Account>("SELECT * FROM accounts ORDER BY created_at DESC")
                .fetch_all(&self.pool)
                .await?;
        for account in &mut rows {
            self.decrypt_account_fields(account)?;
        }
        Ok(rows)
    }

    pub async fn get_account(&self, id: &str) -> Result<Account> {
        let mut account = sqlx::query_as::<_, Account>("SELECT * FROM accounts WHERE id = ?")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| TitenError::AccountNotFound(id.to_string()))?;
        self.decrypt_account_fields(&mut account)?;
        Ok(account)
    }

    pub async fn get_account_by_username(&self, username: &str) -> Result<Account> {
        let mut account = sqlx::query_as::<_, Account>("SELECT * FROM accounts WHERE username = ?")
            .bind(username)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| TitenError::AccountNotFound(username.to_string()))?;
        self.decrypt_account_fields(&mut account)?;
        Ok(account)
    }

    pub async fn create_account(&self, id: &str, input: &CreateAccount) -> Result<Account> {
        let enc_token = self.encrypt_field(&input.access_token)?;
        let enc_secret = match &input.app_secret {
            Some(s) if !s.is_empty() => Some(self.encrypt_field(s)?),
            _ => None,
        };

        sqlx::query(
            "INSERT INTO accounts (id, username, user_id, access_token, expires_at, app_id, app_secret)\n             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(&input.username)
        .bind(&input.user_id)
        .bind(&enc_token)
        .bind(&input.expires_at)
        .bind(&input.app_id)
        .bind(&enc_secret)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                TitenError::AccountAlreadyExists(input.username.clone().unwrap_or_default())
            } else {
                TitenError::DatabaseError(e.to_string())
            }
        })?;

        self.get_account(id).await
    }

    pub async fn update_account(&self, id: &str, input: &UpdateAccount) -> Result<Account> {
        let acc = self.get_account(id).await?;

        let expires_at = input.expires_at.as_deref().unwrap_or(&acc.expires_at);
        let is_active = input.is_active.unwrap_or(acc.is_active);

        if let Some(new_token) = &input.access_token {
            let enc_token = self.encrypt_field(new_token)?;
            sqlx::query(
                "UPDATE accounts SET access_token = ?, expires_at = ?, is_active = ?, updated_at = datetime('now')\n                 WHERE id = ?",
            )
            .bind(&enc_token)
            .bind(expires_at)
            .bind(is_active)
            .bind(id)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                "UPDATE accounts SET expires_at = ?, is_active = ?, updated_at = datetime('now')\n                 WHERE id = ?",
            )
            .bind(expires_at)
            .bind(is_active)
            .bind(id)
            .execute(&self.pool)
            .await?;
        }

        self.get_account(id).await
    }

    pub async fn delete_account(&self, id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM accounts WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(TitenError::AccountNotFound(id.to_string()));
        }
        Ok(())
    }

    // ─── Posts ─────────────────────────────────────────────

    pub async fn list_posts(&self, filter: &PostFilter) -> Result<Vec<Post>> {
        let limit = filter.limit.unwrap_or(50).clamp(1, 1000);
        let offset = filter.offset.unwrap_or(0).max(0);

        let mut query = String::from("SELECT * FROM posts WHERE 1=1");
        if filter.account_id.is_some() {
            query.push_str(" AND account_id = ?");
        }
        if filter.status.is_some() {
            query.push_str(" AND status = ?");
        }
        if filter.media_type.is_some() {
            query.push_str(" AND media_type = ?");
        }
        if filter.from.is_some() {
            query.push_str(" AND created_at >= ?");
        }
        if filter.to.is_some() {
            query.push_str(" AND created_at <= ?");
        }
        if filter.search.is_some() {
            query.push_str(" AND caption LIKE ? COLLATE NOCASE");
        }
        query.push_str(" ORDER BY created_at DESC LIMIT ? OFFSET ?");

        let mut q = sqlx::query_as::<_, Post>(&query);
        if let Some(ref aid) = filter.account_id {
            q = q.bind(aid);
        }
        if let Some(ref s) = filter.status {
            q = q.bind(s);
        }
        if let Some(ref mt) = filter.media_type {
            q = q.bind(mt);
        }
        if let Some(ref f) = filter.from {
            q = q.bind(f);
        }
        if let Some(ref t) = filter.to {
            q = q.bind(t);
        }
        if let Some(ref search) = filter.search {
            q = q.bind(format!("%{search}%"));
        }
        q.bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn get_post(&self, id: &str) -> Result<Post> {
        sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE id = ?")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| TitenError::PostNotFound(id.to_string()))
    }

    pub async fn create_post(&self, id: &str, input: &CreatePost) -> Result<Post> {
        let media_type = input.media_type.as_deref().unwrap_or("TEXT");
        sqlx::query(
            "INSERT INTO posts (id, account_id, media_type, caption, text_attachment, status, published_at)
             VALUES (?, ?, ?, ?, ?, 'published', datetime('now'))",
        )
        .bind(id)
        .bind(&input.account_id)
        .bind(media_type)
        .bind(&input.caption)
        .bind(&input.text_attachment)
        .execute(&self.pool)
        .await?;

        self.get_post(id).await
    }

    /// Create a post record with threads_post_id — used by scheduler and
    /// immediate publish paths where the Threads post ID is known.
    pub async fn create_post_with_threads_id(
        &self,
        id: &str,
        input: &CreatePost,
        threads_post_id: &str,
    ) -> Result<Post> {
        let media_type = input.media_type.as_deref().unwrap_or("TEXT");
        sqlx::query(
            "INSERT INTO posts (id, threads_post_id, account_id, media_type, caption, text_attachment, status, published_at)
             VALUES (?, ?, ?, ?, ?, ?, 'published', datetime('now'))",
        )
        .bind(id)
        .bind(threads_post_id)
        .bind(&input.account_id)
        .bind(media_type)
        .bind(&input.caption)
        .bind(&input.text_attachment)
        .execute(&self.pool)
        .await?;

        self.get_post(id).await
    }

    pub async fn delete_post(&self, id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM posts WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(TitenError::PostNotFound(id.to_string()));
        }
        Ok(())
    }

    // ─── Schedules ──────────────────────────────────────────

    pub async fn list_schedules(&self, filter: &ScheduleFilter) -> Result<Vec<Schedule>> {
        let limit = filter.limit.unwrap_or(50).clamp(1, 1000);
        let offset = filter.offset.unwrap_or(0).max(0);

        let mut query = String::from("SELECT * FROM schedules WHERE 1=1");
        if filter.account_id.is_some() {
            query.push_str(" AND account_id = ?");
        }
        if filter.status.is_some() {
            query.push_str(" AND status = ?");
        }
        if filter.media_type.is_some() {
            query.push_str(" AND media_type = ?");
        }
        if filter.from.is_some() {
            query.push_str(" AND scheduled_at >= ?");
        }
        if filter.to.is_some() {
            query.push_str(" AND scheduled_at <= ?");
        }
        if filter.search.is_some() {
            query.push_str(" AND caption LIKE ? COLLATE NOCASE");
        }
        query.push_str(" ORDER BY scheduled_at ASC LIMIT ? OFFSET ?");

        let mut q = sqlx::query_as::<_, Schedule>(&query);
        if let Some(ref aid) = filter.account_id {
            q = q.bind(aid);
        }
        if let Some(ref s) = filter.status {
            q = q.bind(s);
        }
        if let Some(ref mt) = filter.media_type {
            q = q.bind(mt);
        }
        if let Some(ref f) = filter.from {
            q = q.bind(f);
        }
        if let Some(ref t) = filter.to {
            q = q.bind(t);
        }
        if let Some(ref search) = filter.search {
            q = q.bind(format!("%{search}%"));
        }
        q.bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn get_due_schedules(&self) -> Result<Vec<Schedule>> {
        // #107 fix: Normalize scheduled_at (ISO 8601 with offset) to comparable UTC.
        // datetime('now') returns "YYYY-MM-DD HH:MM:SS" (UTC, space separator).
        // scheduled_at is stored as "YYYY-MM-DDTHH:MM:SS+07:00" (T separator + offset).
        // SQLite lexicographic comparison fails: 'T' (84) > ' ' (32).
        //
        // Fix: use datetime(scheduled_at) to parse ISO 8601 → comparable format,
        // then compare against datetime('now').
        sqlx::query_as::<_, Schedule>(
            "SELECT * FROM schedules \
             WHERE status = 'pending' \
               AND datetime(scheduled_at) <= datetime('now') \
             ORDER BY scheduled_at ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn create_schedule(&self, id: &str, input: &CreateSchedule) -> Result<Schedule> {
        let media_type = input.media_type.as_deref().unwrap_or("TEXT");
        let media_urls = input
            .media_urls
            .as_ref()
            .map(|urls| serde_json::to_string(urls).unwrap_or_default());

        // Sanitize caption: normalize line endings (#82)
        let caption = input
            .caption
            .as_deref()
            .map(crate::models::sanitize_caption);

        // HITL: new schedules default to 'draft' (requires human approval).
        // If auto_approve=true, skip directly to 'pending' for backward compat.
        let status = if input.auto_approve {
            "pending"
        } else {
            "draft"
        };

        sqlx::query(
            "INSERT INTO schedules (id, account_id, media_type, caption, text_attachment, media_urls, scheduled_at, status, location_id)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(&input.account_id)
        .bind(media_type)
        .bind(&caption)
        .bind(&input.text_attachment)
        .bind(&media_urls)
        .bind(&input.scheduled_at)
        .bind(status)
        .bind(&input.location_id)
        .execute(&self.pool)
        .await?;

        self.get_schedule(id).await
    }

    pub async fn get_schedule(&self, id: &str) -> Result<Schedule> {
        sqlx::query_as::<_, Schedule>("SELECT * FROM schedules WHERE id = ?")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| TitenError::ScheduleNotFound(id.to_string()))
    }

    pub async fn update_schedule_status(
        &self,
        id: &str,
        status: &str,
        result_json: Option<&str>,
        error: Option<&str>,
    ) -> Result<()> {
        // #112 fix: When status = 'published', also set published_at and
        // extract result_post_id from result_json for audit trail.
        if status == "published" {
            // Extract threads_post_id from result_json if present
            let result_post_id: Option<String> = result_json
                .and_then(|j| serde_json::from_str::<serde_json::Value>(j).ok())
                .and_then(|v| {
                    v.get("threads_post_id")
                        .and_then(|t| t.as_str())
                        .map(String::from)
                });

            sqlx::query(
                "UPDATE schedules \
                 SET status = ?, result_json = ?, error = ?, published_at = datetime('now'), \
                     result_post_id = COALESCE(?, result_post_id), updated_at = datetime('now') \
                 WHERE id = ?",
            )
            .bind(status)
            .bind(result_json)
            .bind(error)
            .bind(result_post_id)
            .bind(id)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                "UPDATE schedules SET status = ?, result_json = ?, error = ?, updated_at = datetime('now') WHERE id = ?",
            )
            .bind(status)
            .bind(result_json)
            .bind(error)
            .bind(id)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    /// Atomically claim a schedule for processing.
    ///
    /// Only transitions `pending → processing` if the row is still pending.
    /// Returns `true` if claimed, `false` if already claimed by another worker.
    /// This prevents double-posting when multiple scheduler instances run.
    pub async fn claim_schedule(&self, id: &str) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE schedules SET status = 'processing', updated_at = datetime('now') WHERE id = ? AND status = 'pending'",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Reap schedules stuck in `processing` state longer than a timeout.
    ///
    /// If the server crashes or a panic occurs after `claim_schedule` but before
    /// the schedule reaches `published` or `failed`, the row stays `processing`
    /// forever and the post is never retried. This resets rows that have been
    /// `processing` for longer than `stale_secs` back to `pending` so the next
    /// scheduler tick picks them up again.
    pub async fn reap_stale_schedules(&self, stale_secs: i64) -> Result<u64> {
        let result = sqlx::query(
            "UPDATE schedules
             SET status = 'pending', updated_at = datetime('now')
             WHERE status = 'processing'
               AND updated_at < datetime('now', ?)",
        )
        .bind(format!("-{stale_secs} seconds"))
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn delete_schedule(&self, id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM schedules WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(TitenError::ScheduleNotFound(id.to_string()));
        }
        Ok(())
    }

    /// Approve a draft schedule → transitions `draft → pending`.
    /// Only schedules with status 'draft' can be approved.
    /// Returns the updated schedule.
    pub async fn approve_schedule(&self, id: &str, approved_by: Option<&str>) -> Result<Schedule> {
        let result = sqlx::query(
            "UPDATE schedules
             SET status = 'pending', approved_by = ?, approved_at = datetime('now'), updated_at = datetime('now')
             WHERE id = ? AND status = 'draft'",
        )
        .bind(approved_by)
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            // Either not found or not in draft state
            let schedule = self.get_schedule(id).await?;
            return Err(TitenError::InvalidRequest(format!(
                "Schedule {} is in '{}' state, can only approve from 'draft'",
                id, schedule.status
            )));
        }

        self.get_schedule(id).await
    }

    /// Reject a draft schedule → transitions `draft → rejected`.
    /// Only schedules with status 'draft' can be rejected.
    /// Returns the updated schedule.
    pub async fn reject_schedule(&self, id: &str, reason: Option<&str>) -> Result<Schedule> {
        let result = sqlx::query(
            "UPDATE schedules
             SET status = 'rejected', error = ?, updated_at = datetime('now')
             WHERE id = ? AND status = 'draft'",
        )
        .bind(reason)
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            let schedule = self.get_schedule(id).await?;
            return Err(TitenError::InvalidRequest(format!(
                "Schedule {} is in '{}' state, can only reject from 'draft'",
                id, schedule.status
            )));
        }

        self.get_schedule(id).await
    }

    /// Update editable fields of a schedule (caption, media_urls, scheduled_at, media_type).
    /// Only schedules in 'draft' or 'pending' state can be edited.
    pub async fn update_schedule_fields(
        &self,
        id: &str,
        caption: Option<&str>,
        media_type: Option<&str>,
        media_urls: Option<Vec<String>>,
        scheduled_at: Option<&str>,
        location_id: Option<&str>,
    ) -> Result<Schedule> {
        // Serialize media_urls if provided
        let media_urls_str = media_urls
            .as_ref()
            .map(|urls| serde_json::to_string(urls).unwrap_or_default());

        // Sanitize caption: normalize line endings (#82)
        let caption_sanitized = caption.map(crate::models::sanitize_caption);

        // Use COALESCE in SQL: None → keep existing, Some(v) → set new value
        // This allows callers to explicitly clear a field by passing Some("")
        let result = sqlx::query(
            "UPDATE schedules
             SET caption = COALESCE(?, caption),
                 media_type = COALESCE(?, media_type),
                 media_urls = COALESCE(?, media_urls),
                 scheduled_at = COALESCE(?, scheduled_at),
                 location_id = COALESCE(?, location_id),
                 updated_at = datetime('now')
             WHERE id = ? AND status = 'draft'",
        )
        .bind(caption_sanitized.as_deref())
        .bind(media_type)
        .bind(media_urls_str)
        .bind(scheduled_at)
        .bind(location_id)
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            // Re-fetch to determine cause: not found vs wrong status
            return match self.get_schedule(id).await {
                Ok(current) => Err(TitenError::InvalidRequest(format!(
                    "Schedule {} is in '{}' state, can only edit 'draft'",
                    id, current.status
                ))),
                Err(e) if matches!(e, TitenError::ScheduleNotFound(_)) => Err(e),
                Err(e) => Err(e),
            };
        }

        self.get_schedule(id).await
    }

    // ─── Comments ───────────────────────────────────────────

    pub async fn list_comments(
        &self,
        post_id: &str,
        filter: &CommentFilter,
    ) -> Result<Vec<Comment>> {
        let limit = filter.limit.unwrap_or(50).clamp(1, 1000);
        let offset = filter.offset.unwrap_or(0).max(0);

        let mut query = String::from("SELECT * FROM comments WHERE post_id = ?");
        if filter.sentiment.is_some() {
            query.push_str(" AND sentiment = ?");
        }
        if filter.reply_status.is_some() {
            query.push_str(" AND reply_status = ?");
        }
        if filter.from.is_some() {
            query.push_str(" AND fetched_at >= ?");
        }
        if filter.to.is_some() {
            query.push_str(" AND fetched_at <= ?");
        }
        if filter.search.is_some() {
            query.push_str(" AND text LIKE ? COLLATE NOCASE");
        }
        query.push_str(" ORDER BY fetched_at ASC LIMIT ? OFFSET ?");

        let mut q = sqlx::query_as::<_, Comment>(&query).bind(post_id);
        if let Some(ref s) = filter.sentiment {
            q = q.bind(s);
        }
        if let Some(ref rs) = filter.reply_status {
            q = q.bind(rs);
        }
        if let Some(ref f) = filter.from {
            q = q.bind(f);
        }
        if let Some(ref t) = filter.to {
            q = q.bind(t);
        }
        if let Some(ref search) = filter.search {
            q = q.bind(format!("%{search}%"));
        }
        q.bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn insert_comment(
        &self,
        id: &str,
        post_id: &str,
        author_username: Option<&str>,
        author_user_id: Option<&str>,
        text: &str,
    ) -> Result<Comment> {
        sqlx::query(
            "INSERT INTO comments (id, post_id, author_username, author_user_id, text) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(post_id)
        .bind(author_username)
        .bind(author_user_id)
        .bind(text)
        .execute(&self.pool)
        .await?;

        sqlx::query_as::<_, Comment>("SELECT * FROM comments WHERE id = ?")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn update_comment_sentiment(&self, id: &str, label: &str, score: f64) -> Result<()> {
        sqlx::query("UPDATE comments SET sentiment = ?, sentiment_score = ? WHERE id = ?")
            .bind(label)
            .bind(score)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Fetch a single comment by ID.
    pub async fn get_comment(&self, id: &str) -> Result<Comment> {
        sqlx::query_as::<_, Comment>("SELECT * FROM comments WHERE id = ?")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => TitenError::CommentNotFound(id.to_string()),
                other => other.into(),
            })
    }

    /// Update comment reply status and optionally store reply text.
    pub async fn update_comment_reply(
        &self,
        id: &str,
        reply_status: &str,
        reply_text: Option<&str>,
    ) -> Result<Comment> {
        let replied_at = if reply_status == "replied" {
            Some(chrono::Utc::now().to_rfc3339())
        } else {
            None
        };

        let result = sqlx::query(
            "UPDATE comments SET reply_status = ?, reply_text = ?, replied_at = COALESCE(?, replied_at) WHERE id = ?",
        )
        .bind(reply_status)
        .bind(reply_text)
        .bind(&replied_at)
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(TitenError::CommentNotFound(id.to_string()));
        }

        sqlx::query_as::<_, Comment>("SELECT * FROM comments WHERE id = ?")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(Into::into)
    }

    // ─── Mentions ───────────────────────────────────────────

    pub async fn upsert_mention(&self, mention: &Mention) -> Result<Mention> {
        // ON CONFLICT requires non-NULL threads_mention_id; caller must skip
        // mentions without a Threads ID (handled in fetch_mentions handler).
        let conflict_id = match mention.threads_mention_id.as_deref() {
            Some(id) if !id.is_empty() => id,
            _ => {
                return Err(TitenError::InvalidRequest(
                    "upsert_mention requires non-empty threads_mention_id".into(),
                ));
            }
        };
        sqlx::query(
            "INSERT INTO mentions (id, account_id, threads_mention_id, author_username, author_user_id, text, media_type, permalink, mentioned_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(account_id, threads_mention_id) DO UPDATE SET
               author_username = excluded.author_username,
               author_user_id = excluded.author_user_id,
               text = excluded.text,
               media_type = excluded.media_type,
               permalink = excluded.permalink,
               mentioned_at = excluded.mentioned_at,
               fetched_at = datetime('now')",
        )
        .bind(&mention.id)
        .bind(&mention.account_id)
        .bind(conflict_id)
        .bind(&mention.author_username)
        .bind(&mention.author_user_id)
        .bind(&mention.text)
        .bind(&mention.media_type)
        .bind(&mention.permalink)
        .bind(&mention.mentioned_at)
        .execute(&self.pool)
        .await?;

        sqlx::query_as::<_, Mention>(
            "SELECT * FROM mentions WHERE account_id = ? AND threads_mention_id = ?",
        )
        .bind(&mention.account_id)
        .bind(conflict_id)
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn list_mentions(&self, filter: &MentionFilter) -> Result<Vec<Mention>> {
        let limit = filter.limit.unwrap_or(50).clamp(1, 1000);
        let offset = filter.offset.unwrap_or(0).max(0);

        let mut query = String::from("SELECT * FROM mentions WHERE 1=1");

        if filter.account_id.is_some() {
            query.push_str(" AND account_id = ?");
        }
        if filter.date_from.is_some() {
            query.push_str(" AND fetched_at >= ?");
        }
        if filter.date_to.is_some() {
            query.push_str(" AND fetched_at <= ?");
        }
        query.push_str(" ORDER BY fetched_at DESC LIMIT ? OFFSET ?");

        let mut q = sqlx::query_as::<_, Mention>(&query);

        if let Some(ref acct) = filter.account_id {
            q = q.bind(acct);
        }
        if let Some(ref from) = filter.date_from {
            q = q.bind(from);
        }
        if let Some(ref to) = filter.date_to {
            q = q.bind(to);
        }
        q = q.bind(limit).bind(offset);

        q.fetch_all(&self.pool).await.map_err(Into::into)
    }

    // ─── Analytics ───────────────────────────────────────────

    pub async fn insert_analytics_snap(
        &self,
        id: &str,
        post_id: &str,
        insights: &Insights,
    ) -> Result<AnalyticsSnap> {
        sqlx::query(
            "INSERT INTO analytics_snap (id, post_id, likes, replies, reposts, views, quotes)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(post_id)
        .bind(insights.likes.unwrap_or(0))
        .bind(insights.replies.unwrap_or(0))
        .bind(insights.reposts.unwrap_or(0))
        .bind(insights.views.unwrap_or(0))
        .bind(insights.quotes.unwrap_or(0))
        .execute(&self.pool)
        .await?;

        sqlx::query_as::<_, AnalyticsSnap>("SELECT * FROM analytics_snap WHERE id = ?")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn list_analytics_snap(&self, post_id: &str) -> Result<Vec<AnalyticsSnap>> {
        sqlx::query_as::<_, AnalyticsSnap>(
            "SELECT * FROM analytics_snap WHERE post_id = ? ORDER BY snapshot_at ASC",
        )
        .bind(post_id)
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    // ─── Media ──────────────────────────────────────────────

    pub async fn list_media(&self, filter: &MediaFilter) -> Result<Vec<MediaAsset>> {
        let limit = filter.limit.unwrap_or(50).clamp(1, 1000);
        let offset = filter.offset.unwrap_or(0).max(0);

        let mut query = String::from("SELECT * FROM media_assets WHERE 1=1");
        if filter.content_type.is_some() {
            query.push_str(" AND content_type = ?");
        }
        if filter.search.is_some() {
            query.push_str(" AND filename LIKE ? COLLATE NOCASE");
        }
        query.push_str(" ORDER BY uploaded_at DESC LIMIT ? OFFSET ?");

        let mut q = sqlx::query_as::<_, MediaAsset>(&query);
        if let Some(ref ct) = filter.content_type {
            q = q.bind(ct);
        }
        if let Some(ref search) = filter.search {
            q = q.bind(format!("%{search}%"));
        }
        q.bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(Into::into)
    }

    /// Count total media assets matching a filter (for pagination metadata).
    pub async fn count_media(&self, filter: &MediaFilter) -> Result<i64> {
        let mut query = String::from("SELECT COUNT(*) as count FROM media_assets WHERE 1=1");
        if filter.content_type.is_some() {
            query.push_str(" AND content_type = ?");
        }
        if filter.search.is_some() {
            query.push_str(" AND filename LIKE ? COLLATE NOCASE");
        }

        let mut q = sqlx::query_scalar::<_, i64>(&query);
        if let Some(ref ct) = filter.content_type {
            q = q.bind(ct);
        }
        if let Some(ref search) = filter.search {
            q = q.bind(format!("%{search}%"));
        }
        q.fetch_one(&self.pool).await.map_err(Into::into)
    }

    /// Get a single media asset by ID.
    pub async fn get_media_asset(&self, id: &str) -> Result<MediaAsset> {
        sqlx::query_as::<_, MediaAsset>("SELECT * FROM media_assets WHERE id = ?")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn create_media_asset(
        &self,
        id: &str,
        filename: &str,
        content_type: &str,
        size_bytes: i64,
        s3_key: &str,
        s3_url: Option<&str>,
    ) -> Result<MediaAsset> {
        sqlx::query(
            "INSERT INTO media_assets (id, filename, content_type, size_bytes, s3_key, s3_url) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(filename)
        .bind(content_type)
        .bind(size_bytes)
        .bind(s3_key)
        .bind(s3_url)
        .execute(&self.pool)
        .await?;

        sqlx::query_as::<_, MediaAsset>("SELECT * FROM media_assets WHERE id = ?")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn delete_media(&self, id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM media_assets WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(TitenError::MediaNotFound(id.to_string()));
        }
        Ok(())
    }

    // ─── Rate Limiting ──────────────────────────────────────

    pub async fn check_rate_limit(
        &self,
        account_id: &str,
        action: &str,
        limit: i64,
    ) -> Result<i64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(count), 0) FROM rate_tracking WHERE account_id = ? AND action_type = ? AND timestamp > datetime('now', '-24 hours')",
        )
        .bind(account_id)
        .bind(action)
        .fetch_one(&self.pool)
        .await?;

        if count >= limit {
            return Err(TitenError::RateLimitExceeded {
                action: action.to_string(),
                current: count,
                limit,
            });
        }
        Ok(limit - count)
    }

    pub async fn track_rate(&self, account_id: &str, action: &str) -> Result<()> {
        sqlx::query("INSERT INTO rate_tracking (id, account_id, action_type) VALUES (?, ?, ?)")
            .bind(uuid::Uuid::now_v7().to_string())
            .bind(account_id)
            .bind(action)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ── App Settings ───────────────────────────────────────

    /// Load app settings from the database (single-row table, id=1).
    /// Returns encrypted values as-is; callers must decrypt if needed.
    pub async fn get_app_settings(&self) -> Result<AppSettings> {
        let mut settings: AppSettings =
            sqlx::query_as("SELECT instance_name, auto_fetch_comments, comment_fetch_interval, schedule_lookahead_hours, threads_app_id, threads_app_secret_enc, updated_at FROM app_settings WHERE id = 1")
                .fetch_one(&self.pool)
                .await
                .map_err(|e| TitenError::DatabaseError(e.to_string()))?;
        let _ = &mut settings; // suppress unused_mut
        Ok(settings)
    }

    /// Get the decrypted Threads app secret (for server-side OAuth exchange).
    /// Returns `None` if not configured or empty.
    pub async fn get_threads_app_secret(&self) -> Result<Option<String>> {
        let settings = self.get_app_settings().await?;
        match settings.threads_app_secret_enc {
            Some(ref enc) if !enc.is_empty() => {
                let plaintext = self.decrypt_field(enc)?;
                if plaintext.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(plaintext))
                }
            }
            _ => Ok(None),
        }
    }

    /// Update app settings. Only provided fields are changed.
    /// If `threads_app_secret` is `Some(value)`, the value is encrypted before storage.
    /// If `None`, the existing encrypted secret is preserved.
    pub async fn update_app_settings(&self, input: &UpdateAppSettings) -> Result<AppSettings> {
        let current = self.get_app_settings().await?;

        let instance_name = input.instance_name.clone().unwrap_or(current.instance_name);
        let auto_fetch = input
            .auto_fetch_comments
            .unwrap_or(current.auto_fetch_comments);
        let comment_interval = input
            .comment_fetch_interval
            .clone()
            .unwrap_or(current.comment_fetch_interval);
        let lookahead = input
            .schedule_lookahead_hours
            .clone()
            .unwrap_or(current.schedule_lookahead_hours);
        let app_id = input.threads_app_id.clone().or(current.threads_app_id);

        // Handle secret: encrypt if new value provided, otherwise keep existing
        let secret_enc = match &input.threads_app_secret {
            Some(val) if !val.is_empty() => Some(self.encrypt_field(val)?),
            Some(_) => None, // empty string = clear secret
            None => current.threads_app_secret_enc.clone(), // keep existing
        };

        sqlx::query(
            "UPDATE app_settings SET instance_name = ?, auto_fetch_comments = ?, comment_fetch_interval = ?, schedule_lookahead_hours = ?, threads_app_id = ?, threads_app_secret_enc = ?, updated_at = datetime('now') WHERE id = 1",
        )
        .bind(&instance_name)
        .bind(auto_fetch)
        .bind(&comment_interval)
        .bind(&lookahead)
        .bind(&app_id)
        .bind(&secret_enc)
        .execute(&self.pool)
        .await?;

        self.get_app_settings().await
    }
}
