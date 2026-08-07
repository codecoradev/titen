use crate::crypto::Cipher;
use crate::error::{Result, TitenError};
use crate::models::*;
use sqlx::SqlitePool;

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
                panic!(
                    "TITEN_REQUIRE_ENCRYPTION is set but TITEN_ENCRYPTION_KEY is invalid: {e}"
                );
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
        let sql = include_str!("../../titen-api/migrations/001_initial.sql");
        for statement in sql.split(';') {
            let trimmed = statement.trim();
            if !trimmed.is_empty() {
                sqlx::query(trimmed).execute(&self.pool).await?;
            }
        }

        // 002 — drop refresh_token column (safe for fresh installs: ignore "no such column")
        let sql_002 = include_str!("../../titen-api/migrations/002_drop_refresh_token.sql");
        for statement in sql_002.split(';') {
            let trimmed = statement.trim();
            if !trimmed.is_empty() {
                let result = sqlx::query(trimmed).execute(&self.pool).await;
                if let Err(e) = result {
                    let msg = e.to_string();
                    // SQLite error: "no such column" means the column was already absent (fresh install)
                    if !msg.contains("no such column") {
                        return Err(TitenError::DatabaseError(msg));
                    }
                }
            }
        }

        // 003 — add app_secret column (safe for fresh installs: ignore "duplicate column")
        let sql_003 = include_str!("../../titen-api/migrations/003_add_app_secret.sql");
        for statement in sql_003.split(';') {
            let trimmed = statement.trim();
            if !trimmed.is_empty() {
                let result = sqlx::query(trimmed).execute(&self.pool).await;
                if let Err(e) = result {
                    let msg = e.to_string();
                    if !msg.contains("duplicate column") {
                        return Err(TitenError::DatabaseError(msg));
                    }
                }
            }
        }

        // 004 — encryption metadata table
        let sql_004 = include_str!("../../titen-api/migrations/004_encrypt_tokens.sql");
        for statement in sql_004.split(';') {
            let trimmed = statement.trim();
            if !trimmed.is_empty() {
                sqlx::query(trimmed).execute(&self.pool).await?;
            }
        }

        // 004 (Rust side) — encrypt existing plaintext tokens if cipher is available
        self.migrate_encrypted_fields().await?;

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

    pub async fn list_posts(
        &self,
        account_id: Option<&str>,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Post>> {
        let mut query = String::from("SELECT * FROM posts WHERE 1=1");
        if account_id.is_some() {
            query.push_str(" AND account_id = ?");
        }
        if status.is_some() {
            query.push_str(" AND status = ?");
        }
        query.push_str(" ORDER BY created_at DESC LIMIT ? OFFSET ?");

        let mut q = sqlx::query_as::<_, Post>(&query);
        if let Some(aid) = account_id {
            q = q.bind(aid);
        }
        if let Some(s) = status {
            q = q.bind(s);
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
            "INSERT INTO posts (id, account_id, media_type, caption, text_attachment, status)
             VALUES (?, ?, ?, ?, ?, 'published')",
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

    pub async fn list_schedules(
        &self,
        account_id: Option<&str>,
        status: Option<&str>,
    ) -> Result<Vec<Schedule>> {
        let mut query = String::from("SELECT * FROM schedules WHERE 1=1");
        if account_id.is_some() {
            query.push_str(" AND account_id = ?");
        }
        if status.is_some() {
            query.push_str(" AND status = ?");
        }
        query.push_str(" ORDER BY scheduled_at ASC");

        let mut q = sqlx::query_as::<_, Schedule>(&query);
        if let Some(aid) = account_id {
            q = q.bind(aid);
        }
        if let Some(s) = status {
            q = q.bind(s);
        }
        q.fetch_all(&self.pool).await.map_err(Into::into)
    }

    pub async fn get_due_schedules(&self) -> Result<Vec<Schedule>> {
        sqlx::query_as::<_, Schedule>(
            "SELECT * FROM schedules WHERE status = 'pending' AND scheduled_at <= datetime('now') ORDER BY scheduled_at ASC",
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

        sqlx::query(
            "INSERT INTO schedules (id, account_id, media_type, caption, text_attachment, media_urls, scheduled_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(&input.account_id)
        .bind(media_type)
        .bind(&input.caption)
        .bind(&input.text_attachment)
        .bind(&media_urls)
        .bind(&input.scheduled_at)
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
        sqlx::query(
            "UPDATE schedules SET status = ?, result_json = ?, error = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(status)
        .bind(result_json)
        .bind(error)
        .bind(id)
        .execute(&self.pool)
        .await?;
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

    // ─── Comments ───────────────────────────────────────────

    pub async fn list_comments(&self, post_id: &str) -> Result<Vec<Comment>> {
        sqlx::query_as::<_, Comment>(
            "SELECT * FROM comments WHERE post_id = ? ORDER BY fetched_at ASC",
        )
        .bind(post_id)
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

    pub async fn list_media(&self) -> Result<Vec<MediaAsset>> {
        sqlx::query_as::<_, MediaAsset>("SELECT * FROM media_assets ORDER BY uploaded_at DESC")
            .fetch_all(&self.pool)
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
}
