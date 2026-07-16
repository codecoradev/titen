use crate::error::{Result, TitenError};
use crate::models::*;
use sqlx::SqlitePool;

/// Main store — SQLite database access for all titen entities
pub struct Store {
    pool: SqlitePool,
}

impl Store {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Run migrations from embedded SQL
    pub async fn migrate(&self) -> Result<()> {
        let sql = include_str!("../../titen-api/migrations/001_initial.sql");
        for statement in sql.split(';') {
            let trimmed = statement.trim();
            if !trimmed.is_empty() {
                sqlx::query(trimmed).execute(&self.pool).await?;
            }
        }
        Ok(())
    }

    // ─── Accounts ───────────────────────────────────────────

    pub async fn list_accounts(&self) -> Result<Vec<Account>> {
        let rows = sqlx::query_as::<_, Account>("SELECT * FROM accounts ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    pub async fn get_account(&self, id: &str) -> Result<Account> {
        sqlx::query_as::<_, Account>("SELECT * FROM accounts WHERE id = ?")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| TitenError::AccountNotFound(id.to_string()))
    }

    pub async fn get_account_by_username(&self, username: &str) -> Result<Account> {
        sqlx::query_as::<_, Account>("SELECT * FROM accounts WHERE username = ?")
            .bind(username)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| TitenError::AccountNotFound(username.to_string()))
    }

    pub async fn create_account(&self, id: &str, input: &CreateAccount) -> Result<Account> {
        sqlx::query(
            "INSERT INTO accounts (id, username, user_id, access_token, refresh_token, expires_at, app_id)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(&input.username)
        .bind(&input.user_id)
        .bind(&input.access_token)
        .bind(&input.refresh_token)
        .bind(&input.expires_at)
        .bind(&input.app_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                TitenError::AccountAlreadyExists(input.username.clone())
            } else {
                TitenError::DatabaseError(e.to_string())
            }
        })?;

        self.get_account(id).await
    }

    pub async fn update_account(&self, id: &str, input: &UpdateAccount) -> Result<Account> {
        let acc = self.get_account(id).await?;

        let access_token = input.access_token.as_deref().unwrap_or(&acc.access_token);
        let expires_at = input.expires_at.as_deref().unwrap_or(&acc.expires_at);
        let refresh_token = input
            .refresh_token
            .as_deref()
            .unwrap_or(acc.refresh_token.as_deref().unwrap_or(""));
        let is_active = input.is_active.unwrap_or(acc.is_active);

        sqlx::query(
            "UPDATE accounts SET access_token = ?, refresh_token = ?, expires_at = ?, is_active = ?, updated_at = datetime('now')
             WHERE id = ?",
        )
        .bind(access_token)
        .bind(refresh_token)
        .bind(expires_at)
        .bind(is_active)
        .bind(id)
        .execute(&self.pool)
        .await?;

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
        if let Some(aid) = account_id {
            query.push_str(&format!(" AND account_id = '{aid}'"));
        }
        if let Some(s) = status {
            query.push_str(&format!(" AND status = '{s}'"));
        }
        query.push_str(" ORDER BY created_at DESC LIMIT ? OFFSET ?");

        sqlx::query_as::<_, Post>(&query)
            .bind(limit)
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
        if let Some(aid) = account_id {
            query.push_str(&format!(" AND account_id = '{aid}'"));
        }
        if let Some(s) = status {
            query.push_str(&format!(" AND status = '{s}'"));
        }
        query.push_str(" ORDER BY scheduled_at ASC");

        sqlx::query_as::<_, Schedule>(&query)
            .fetch_all(&self.pool)
            .await
            .map_err(Into::into)
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
