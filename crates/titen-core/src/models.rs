/// Core models for titen
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ─── Account ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Account {
    pub id: String,
    pub username: String,
    pub user_id: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: String,
    pub app_id: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateAccount {
    pub username: String,
    pub user_id: String,
    pub access_token: String,
    pub expires_at: String,
    pub refresh_token: Option<String>,
    pub app_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAccount {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
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

// ─── Post ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
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

#[derive(Debug, Deserialize)]
pub struct CreatePost {
    pub account_id: String,
    pub media_type: Option<String>,
    pub caption: Option<String>,
    pub text_attachment: Option<String>,
    pub image_url: Option<String>,
    pub video_url: Option<String>,
    pub image_urls: Option<Vec<String>>,
    pub alt_text: Option<String>,
}

// ─── Schedule ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Schedule {
    pub id: String,
    pub account_id: String,
    pub media_type: String,
    pub caption: Option<String>,
    pub text_attachment: Option<String>,
    pub media_urls: Option<String>,
    pub scheduled_at: String,
    pub status: String,
    pub published_at: Option<String>,
    pub result_post_id: Option<String>,
    pub result_json: Option<String>,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateSchedule {
    pub account_id: String,
    pub media_type: Option<String>,
    pub caption: Option<String>,
    pub text_attachment: Option<String>,
    pub media_urls: Option<Vec<String>>,
    pub scheduled_at: String,
}

// ─── Comment ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Comment {
    pub id: String,
    pub post_id: String,
    pub threads_comment_id: Option<String>,
    pub author_username: Option<String>,
    pub author_user_id: Option<String>,
    pub text: String,
    pub sentiment: Option<String>,
    pub sentiment_score: Option<f64>,
    pub fetched_at: String,
}

// ─── Analytics ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Insights {
    pub likes: Option<i64>,
    pub replies: Option<i64>,
    pub reposts: Option<i64>,
    pub views: Option<i64>,
    pub quotes: Option<i64>,
}

// ─── Media ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MediaAsset {
    pub id: String,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub s3_key: String,
    pub s3_url: Option<String>,
    pub uploaded_at: String,
}

// ─── Sentiment ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentResult {
    pub label: String,
    pub score: f64,
}

#[derive(Debug, Serialize)]
pub struct SentimentSummary {
    pub total: i64,
    pub positive: i64,
    pub negative: i64,
    pub neutral: i64,
    pub average_score: f64,
}

// ─── Rate Limiting ─────────────────────────────────────────

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
