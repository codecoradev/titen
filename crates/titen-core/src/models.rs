/// Core models for titen
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ─── Account ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct Account {
    pub id: String,
    pub username: String,
    pub user_id: String,
    pub access_token: String,
    pub expires_at: String,
    pub app_id: Option<String>,
    pub app_secret: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAccount {
    pub username: Option<String>,
    pub user_id: Option<String>,
    pub access_token: String,
    pub expires_at: String,
    pub app_id: Option<String>,
    pub app_secret: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateAccount {
    pub access_token: Option<String>,
    pub expires_at: Option<String>,
    pub is_active: Option<bool>,
}

impl Account {
    pub fn token_status(&self) -> &'static str {
        let expires = match DateTime::parse_from_rfc3339(&self.expires_at) {
            Ok(dt) => dt.with_timezone(&Utc),
            Err(_) => return "unknown",
        };
        let now = Utc::now();
        let days_left = (expires - now).num_days();
        match days_left {
            d if d <= 0 => "expired",
            d if d <= 7 => "expiring_soon",
            _ => "valid",
        }
    }
}

// ─── Sanitizers ────────────────────────────────────────────

/// Sanitize caption text: normalize line endings, strip control chars.
/// Issue #82: raw curl with unescaped multiline breaks JSON parsing.
pub fn sanitize_caption(s: &str) -> String {
    s.replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\0', "")
        .trim_end_matches('\n')
        .to_string()
}

// ─── Post ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct Post {
    pub id: String,
    pub threads_post_id: Option<String>,
    pub account_id: String,
    pub media_type: String,
    pub caption: Option<String>,
    pub text_attachment: Option<String>,
    pub carousel_children: Option<String>,
    pub status: String,
    pub scheduled_id: Option<String>,
    pub published_at: Option<String>,
    pub insights_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePost {
    pub account_id: String,
    pub media_type: Option<String>,
    pub caption: Option<String>,
    pub text_attachment: Option<String>,
    pub image_url: Option<String>,
    pub video_url: Option<String>,
    pub image_urls: Option<Vec<String>>,
    /// Media library asset IDs — resolved to S3 URLs for CAROUSEL posts.
    /// Allows users to select from uploaded media instead of pasting URLs.
    pub media_ids: Option<Vec<String>>,
    pub alt_text: Option<String>,
}

// ─── Schedule ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct Schedule {
    pub id: String,
    pub account_id: String,
    pub media_type: String,
    pub caption: Option<String>,
    pub text_attachment: Option<String>,
    /// JSON-encoded array of media URLs (TEXT column, not native JSON).
    /// Decode with `serde_json::from_str::<Vec<String>>` before use.
    pub media_urls: Option<String>,
    pub scheduled_at: String,
    pub status: String,
    pub published_at: Option<String>,
    pub result_post_id: Option<String>,
    pub result_json: Option<String>,
    pub error: Option<String>,
    pub approved_by: Option<String>,
    pub approved_at: Option<String>,
    /// Optional Threads location ID for location tagging.
    pub location_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSchedule {
    pub account_id: String,
    pub media_type: Option<String>,
    pub caption: Option<String>,
    pub text_attachment: Option<String>,
    pub media_urls: Option<Vec<String>>,
    pub scheduled_at: String,
    /// Optional Threads location ID for location tagging.
    pub location_id: Option<String>,
    /// Skip draft state and go straight to 'pending' (auto-approve).
    /// Default: false — all new schedules require human approval.
    #[serde(default)]
    pub auto_approve: bool,
}

/// Partial update for a schedule (HITL edit before approval).
/// All fields optional — only provided fields are updated.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateSchedule {
    pub caption: Option<String>,
    pub media_type: Option<String>,
    pub media_urls: Option<Vec<String>>,
    pub scheduled_at: Option<String>,
    pub location_id: Option<String>,
}

// ─── Comment ──────────────────────────────────────────────

/// Request body for updating comment reply status.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateCommentReply {
    pub reply_status: Option<String>, // new | needs_reply | replied | skipped
    pub reply_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct Comment {
    pub id: String,
    pub post_id: String,
    pub threads_comment_id: Option<String>,
    pub author_username: Option<String>,
    pub author_user_id: Option<String>,
    pub text: String,
    pub sentiment: Option<String>,
    pub sentiment_score: Option<f64>,
    pub reply_status: String,
    pub replied_at: Option<String>,
    pub reply_text: Option<String>,
    pub assigned_priority: i64,
    pub fetched_at: String,
}

// ─── Mentions ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct Mention {
    pub id: String,
    pub account_id: String,
    pub threads_mention_id: Option<String>,
    pub author_username: Option<String>,
    pub author_user_id: Option<String>,
    pub text: Option<String>,
    pub media_type: Option<String>,
    pub permalink: Option<String>,
    pub mentioned_at: Option<String>,
    pub fetched_at: String,
}

// ─── Analytics ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct AnalyticsSnap {
    pub id: String,
    pub post_id: String,
    pub likes: i64,
    pub replies: i64,
    pub reposts: i64,
    pub views: i64,
    pub quotes: i64,
    pub snapshot_at: String,
}

#[derive(Debug, Serialize, Deserialize, Default, ToSchema)]
pub struct Insights {
    pub likes: Option<i64>,
    pub replies: Option<i64>,
    pub reposts: Option<i64>,
    pub views: Option<i64>,
    pub quotes: Option<i64>,
    pub shares: Option<i64>,
}

impl From<Vec<InsightMetric>> for Insights {
    fn from(metrics: Vec<InsightMetric>) -> Self {
        let mut insights = Insights::default();
        for m in metrics {
            let val = m
                .values
                .and_then(|v| v.first().map(|iv| iv.value))
                .or(m.total_value.map(|tv| tv.value));
            match m.name.as_str() {
                "likes" => insights.likes = val,
                "replies" => insights.replies = val,
                "reposts" => insights.reposts = val,
                "views" => insights.views = val,
                "quotes" => insights.quotes = val,
                "shares" => insights.shares = val,
                _ => {}
            }
        }
        insights
    }
}

// ─── Media ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct MediaAsset {
    pub id: String,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub s3_key: String,
    pub s3_url: Option<String>,
    pub uploaded_at: String,
}

// ─── Unified Query Filters (Issue #83) ─────────────────────

/// Filter for post listings — supports date range, media_type, and text search.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PostFilter {
    pub account_id: Option<String>,
    pub status: Option<String>,
    pub media_type: Option<String>,
    /// ISO-8601 — only posts created at or after this date
    pub from: Option<String>,
    /// ISO-8601 — only posts created at or before this date
    pub to: Option<String>,
    /// Case-insensitive LIKE search on caption
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl Default for PostFilter {
    fn default() -> Self {
        Self {
            account_id: None,
            status: None,
            media_type: None,
            from: None,
            to: None,
            search: None,
            limit: Some(50),
            offset: Some(0),
        }
    }
}

/// Filter for schedule listings.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ScheduleFilter {
    pub account_id: Option<String>,
    pub status: Option<String>,
    pub media_type: Option<String>,
    /// ISO-8601 — only schedules at or after this date
    pub from: Option<String>,
    /// ISO-8601 — only schedules at or before this date
    pub to: Option<String>,
    /// Case-insensitive LIKE search on caption
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl Default for ScheduleFilter {
    fn default() -> Self {
        Self {
            account_id: None,
            status: None,
            media_type: None,
            from: None,
            to: None,
            search: None,
            limit: Some(50),
            offset: Some(0),
        }
    }
}

/// Filter for comment listings.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CommentFilter {
    pub sentiment: Option<String>,
    pub reply_status: Option<String>,
    /// ISO-8601 — only comments fetched at or after this date
    pub from: Option<String>,
    /// ISO-8601 — only comments fetched at or before this date
    pub to: Option<String>,
    /// Case-insensitive LIKE search on text
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl Default for CommentFilter {
    fn default() -> Self {
        Self {
            sentiment: None,
            reply_status: None,
            from: None,
            to: None,
            search: None,
            limit: Some(50),
            offset: Some(0),
        }
    }
}

/// Filter for media listings.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct MediaFilter {
    pub content_type: Option<String>,
    /// Case-insensitive LIKE search on filename
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl Default for MediaFilter {
    fn default() -> Self {
        Self {
            content_type: None,
            search: None,
            limit: Some(50),
            offset: Some(0),
        }
    }
}

/// Filter for account listings.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AccountFilter {
    pub is_active: Option<bool>,
    /// Case-insensitive LIKE search on username
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl Default for AccountFilter {
    fn default() -> Self {
        Self {
            is_active: None,
            search: None,
            limit: Some(50),
            offset: Some(0),
        }
    }
}

/// Filter for mention listings.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct MentionFilter {
    pub account_id: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl Default for MentionFilter {
    fn default() -> Self {
        Self {
            account_id: None,
            date_from: None,
            date_to: None,
            limit: Some(50),
            offset: Some(0),
        }
    }
}

// ─── Sentiment ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SentimentResult {
    pub label: String,
    pub score: f64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SentimentSummary {
    pub total: i64,
    pub positive: i64,
    pub negative: i64,
    pub neutral: i64,
    pub average_score: f64,
}

// ─── Rate Limiting ─────────────────────────────────────────

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct RateLimits {
    pub post: i64,
    pub reply: i64,
    pub delete: i64,
}

impl Default for RateLimits {
    fn default() -> Self {
        Self {
            post: 250,
            reply: 1000,
            delete: 100,
        }
    }
}

// ─── Comment Data (from Threads API) ───────────────────────

/// Comment data fetched from the Threads API (before storage)
#[derive(Debug, Clone, ToSchema)]
pub struct CommentData {
    pub threads_comment_id: String,
    pub author_username: Option<String>,
    pub author_user_id: Option<String>,
    pub text: String,
    pub timestamp: Option<String>,
}

// ─── Container Status (Threads API) ─────────────────────────
/// Response from GET /{container_id}?fields=status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContainerStatus {
    pub id: String,
    /// "FINISHED", "IN_PROGRESS", "ERROR"
    pub status: Option<String>,
}

// ─── User Profile (Threads API) ───────────────────────────
/// Response from GET /{user_id}?fields=username,name,...
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserProfile {
    pub id: String,
    pub username: Option<String>,
    pub name: Option<String>,
    pub profile_picture_url: Option<String>,
    pub threads_profile_picture_url: Option<String>,
    pub threads_biography: Option<String>,
    /// Followers count — merged from `threads_insights` (not available on profile node).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followers_count: Option<i64>,
}

// ─── Publishing Limit (Threads API) ─────────────────────────
/// Response from GET /{user_id}/threads_publishing_limit
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublishingLimit {
    pub quota_usage: i64,
    #[serde(default)]
    pub config: Option<PublishingLimitConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublishingLimitConfig {
    pub quota_total: i64,
}

// ─── Insights (official Threads API format) ────────────────
/// Response from GET /{media_id}/insights?metric=likes,reposts,...
/// Returns an array of metrics with per-period values or totals.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InsightMetric {
    pub name: String,
    pub period: String,
    pub values: Option<Vec<InsightValue>>,
    pub total_value: Option<InsightTotalValue>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InsightValue {
    pub value: i64,
    pub end_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InsightTotalValue {
    pub value: i64,
}

// ─── User Insights (Threads API) ───────────────────────────
/// Response from GET /{user_id}/threads_insights?metric=views,likes,...
/// Similar to InsightMetric but includes link_total_values for click metrics.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserInsightMetric {
    pub name: String,
    pub period: String,
    pub values: Option<Vec<InsightValue>>,
    pub total_value: Option<InsightTotalValue>,
    pub link_total_values: Option<Vec<LinkTotalValue>>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LinkTotalValue {
    pub value: i64,
    pub link_url: Option<String>,
}

// ─── Reply Creation (Threads API) ──────────────────────────
/// Request body for POST /{post_id}/replies (creating a reply to a post)
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateReply {
    pub media_type: String,
    pub text: String,
    #[allow(dead_code)]
    pub reply_to_id: Option<String>,
}

// ─── AppSettings ──────────────────────────────────────────

/// Server-side application settings stored in the database.
/// Sensitive fields are encrypted at rest via AES-256-GCM.
///
/// The `threads_app_secret` field is never returned to the client in plaintext.
/// Use [`AppSettingsResponse`] for API responses.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AppSettings {
    pub instance_name: String,
    pub auto_fetch_comments: bool,
    pub comment_fetch_interval: String,
    pub schedule_lookahead_hours: String,
    pub threads_app_id: Option<String>,
    /// Encrypted value as stored in DB (`enc:v1:...` or plaintext).
    /// Not serialized to avoid accidental leakage.
    #[serde(skip)]
    pub threads_app_secret_enc: Option<String>,
    pub updated_at: String,
}

/// Safe representation of settings for API responses.
/// The app secret is masked to indicate presence without revealing the value.
#[derive(Debug, Serialize, ToSchema)]
pub struct AppSettingsResponse {
    pub instance_name: String,
    pub auto_fetch_comments: bool,
    pub comment_fetch_interval: String,
    pub schedule_lookahead_hours: String,
    pub threads_app_id: Option<String>,
    /// `true` if a secret is stored, `false` otherwise.
    /// The actual secret is never sent to the client.
    pub threads_app_secret_set: bool,
}

/// Input for updating settings. All fields optional for partial updates.
/// `threads_app_secret` is only set when the user provides a new value.
/// If `None`, the existing secret is preserved.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateAppSettings {
    pub instance_name: Option<String>,
    pub auto_fetch_comments: Option<bool>,
    pub comment_fetch_interval: Option<String>,
    pub schedule_lookahead_hours: Option<String>,
    pub threads_app_id: Option<String>,
    /// If `Some(value)`, update the secret. If `None`, keep existing.
    /// Empty string `Some("")` clears the secret.
    pub threads_app_secret: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Helper ────────────────────────────────────────────

    fn make_account(expires_at: &str) -> Account {
        Account {
            id: "test-id".into(),
            username: "testuser".into(),
            user_id: "threads-123".into(),
            access_token: "fake-token".into(),
            expires_at: expires_at.into(),
            app_id: Some("app-1".into()),
            app_secret: None,
            is_active: true,
            created_at: "2025-01-01T00:00:00Z".into(),
            updated_at: "2025-01-01T00:00:00Z".into(),
        }
    }

    // ─── Account::token_status ────────────────────────────

    #[test]
    fn token_status_valid() {
        let future = Utc::now() + chrono::Duration::days(30);
        let account = make_account(&future.to_rfc3339());
        assert_eq!(account.token_status(), "valid");
    }

    #[test]
    fn token_status_expiring_soon_7_days() {
        // Exactly 7 days from now → expiring_soon (d <= 7)
        let future = Utc::now() + chrono::Duration::days(7);
        let account = make_account(&future.to_rfc3339());
        assert_eq!(account.token_status(), "expiring_soon");
    }

    #[test]
    fn token_status_expiring_soon_1_day() {
        // 3 days from now → expiring_soon (d <= 7)
        let future = Utc::now() + chrono::Duration::days(3);
        let account = make_account(&future.to_rfc3339());
        assert_eq!(account.token_status(), "expiring_soon");
    }

    #[test]
    fn token_status_expired() {
        let past = Utc::now() - chrono::Duration::days(1);
        let account = make_account(&past.to_rfc3339());
        assert_eq!(account.token_status(), "expired");
    }

    #[test]
    fn token_status_expired_exactly_now() {
        let now = Utc::now();
        let account = make_account(&now.to_rfc3339());
        assert_eq!(account.token_status(), "expired");
    }

    #[test]
    fn token_status_invalid_date() {
        let account = make_account("not-a-valid-date");
        assert_eq!(account.token_status(), "unknown");
    }

    #[test]
    fn token_status_empty_date() {
        let account = make_account("");
        assert_eq!(account.token_status(), "unknown");
    }

    // ─── RateLimits::default ─────────────────────────────

    #[test]
    fn rate_limits_default_values() {
        let limits = RateLimits::default();
        assert_eq!(limits.post, 250);
        assert_eq!(limits.reply, 1000);
        assert_eq!(limits.delete, 100);
    }
}
