use thiserror::Error;

/// Unified error type for titen-core
#[derive(Debug, Error)]
pub enum TitenError {
    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Account already exists: {0}")]
    AccountAlreadyExists(String),

    #[error("Post not found: {0}")]
    PostNotFound(String),

    #[error("Schedule not found: {0}")]
    ScheduleNotFound(String),

    #[error("Media not found: {0}")]
    MediaNotFound(String),

    #[error("Comment not found: {0}")]
    CommentNotFound(String),

    #[error("Token expired for account: {0}")]
    TokenExpired(String),

    #[error("Token refresh failed: {0}")]
    TokenRefreshFailed(String),

    #[error("Rate limit exceeded: {action} ({current}/{limit} in 24h)")]
    RateLimitExceeded {
        action: String,
        current: i64,
        limit: i64,
    },

    #[error("Threads API error: {0}")]
    ThreadsApiError(String),

    #[error("S3 error: {0}")]
    StorageError(String),

    #[error("Sentiment engine error: {0}")]
    SentimentError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

impl From<sqlx::Error> for TitenError {
    fn from(err: sqlx::Error) -> Self {
        TitenError::DatabaseError(err.to_string())
    }
}

impl From<reqwest::Error> for TitenError {
    fn from(err: reqwest::Error) -> Self {
        TitenError::ThreadsApiError(err.to_string())
    }
}

impl TitenError {
    /// Heuristic: does this error message look like a TRANSIENT Threads API
    /// failure worth a bounded scheduler retry (#257)?
    ///
    /// Message-based (not enum-based) because transient errors arrive wrapped:
    /// the publisher wraps carousel child-container failures into
    /// `InvalidRequest("Failed to create carousel item: Threads API error: …")`,
    /// so the OAuthException code only survives in the rendered text.
    ///
    /// Transient: OAuthException #1 (Meta catch-all "unknown error" — observed
    /// killing carousel child creation on 2026-09-08 and 2026-09-10 while TEXT
    /// and IMAGE kept publishing), OAuthException #24 ("resource does not
    /// exist" — observed transient on 2026-08-25), and server-side HTTP
    /// statuses (429/5xx).
    /// Permanent: #100 (invalid parameter — our payload is wrong), #190
    /// (auth/token), validation and caption errors.
    pub fn is_transient_message(msg: &str) -> bool {
        if msg.contains("[OAuthException #1]") || msg.contains("[OAuthException #24]") {
            return true;
        }
        // Server-side HTTP failures surfaced by the threads_post/threads_get
        // helpers as "(HTTP <status>)" in the message.
        for status in [
            "(HTTP 429",
            "(HTTP 500",
            "(HTTP 502",
            "(HTTP 503",
            "(HTTP 504",
        ] {
            if msg.contains(status) {
                return true;
            }
        }
        false
    }
}

pub type Result<T> = std::result::Result<T, TitenError>;

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Display messages ─────────────────────────────────

    #[test]
    fn display_account_not_found() {
        let err = TitenError::AccountNotFound("user123".into());
        let msg = err.to_string();
        assert!(msg.contains("Account not found"));
        assert!(msg.contains("user123"));
    }

    // ─── #257 transient-error classifier ──────────────────

    #[test]
    fn transient_oauth_exception_1() {
        let msg = "Failed to create carousel item 1/5 (https://x/slide-1.jpg): \
                   Threads API error: An unknown error occurred [OAuthException #1] \
                   (HTTP 400 Bad Request)";
        assert!(TitenError::is_transient_message(msg));
    }

    #[test]
    fn transient_oauth_exception_24() {
        let msg = "Threads API error: The requested resource does not exist [OAuthException #24] \
             (HTTP 400 Bad Request)";
        assert!(TitenError::is_transient_message(msg));
    }

    #[test]
    fn transient_http_500() {
        let msg = "Publish failed: Threads API error: (HTTP 500 Internal Server Error)";
        assert!(TitenError::is_transient_message(msg));
    }

    #[test]
    fn transient_http_429() {
        let msg = "Threads API error: rate limited (HTTP 429 Too Many Requests)";
        assert!(TitenError::is_transient_message(msg));
    }

    #[test]
    fn permanent_oauth_exception_100() {
        let msg = "Threads API error: Invalid parameter [OAuthException #100] \
                   (HTTP 400 Bad Request)";
        assert!(!TitenError::is_transient_message(msg));
    }

    #[test]
    fn permanent_validation_error() {
        let msg = "image_url is required for IMAGE posts";
        assert!(!TitenError::is_transient_message(msg));
    }

    #[test]
    fn permanent_caption_error() {
        let msg = "caption exceeds the 499-character limit (CAPTION_TOO_LONG)";
        assert!(!TitenError::is_transient_message(msg));
    }

    #[test]
    fn display_account_already_exists() {
        let err = TitenError::AccountAlreadyExists("user123".into());
        let msg = err.to_string();
        assert!(msg.contains("Account already exists"));
        assert!(msg.contains("user123"));
    }

    #[test]
    fn display_post_not_found() {
        let err = TitenError::PostNotFound("post99".into());
        assert!(err.to_string().contains("Post not found"));
    }

    #[test]
    fn display_schedule_not_found() {
        let err = TitenError::ScheduleNotFound("sched1".into());
        assert!(err.to_string().contains("Schedule not found"));
    }

    #[test]
    fn display_media_not_found() {
        let err = TitenError::MediaNotFound("media1".into());
        assert!(err.to_string().contains("Media not found"));
    }

    #[test]
    fn display_comment_not_found() {
        let err = TitenError::CommentNotFound("cmt1".into());
        assert!(err.to_string().contains("Comment not found"));
    }

    #[test]
    fn display_token_expired() {
        let err = TitenError::TokenExpired("acc1".into());
        let msg = err.to_string();
        assert!(msg.contains("Token expired"));
        assert!(msg.contains("acc1"));
    }

    #[test]
    fn display_token_refresh_failed() {
        let err = TitenError::TokenRefreshFailed("acc1".into());
        let msg = err.to_string();
        assert!(msg.contains("Token refresh failed"));
        assert!(msg.contains("acc1"));
    }

    #[test]
    fn display_rate_limit_exceeded() {
        let err = TitenError::RateLimitExceeded {
            action: "post".into(),
            current: 250,
            limit: 250,
        };
        let msg = err.to_string();
        assert!(msg.contains("Rate limit exceeded"));
        assert!(msg.contains("post"));
        assert!(msg.contains("250"));
    }

    #[test]
    fn display_threads_api_error() {
        let err = TitenError::ThreadsApiError("timeout".into());
        assert!(err.to_string().contains("Threads API error"));
    }

    #[test]
    fn display_storage_error() {
        let err = TitenError::StorageError("bucket full".into());
        assert!(err.to_string().contains("S3 error"));
    }

    #[test]
    fn display_sentiment_error() {
        let err = TitenError::SentimentError("engine failure".into());
        assert!(err.to_string().contains("Sentiment engine error"));
    }

    #[test]
    fn display_database_error() {
        let err = TitenError::DatabaseError("connection lost".into());
        assert!(err.to_string().contains("Database error"));
    }

    #[test]
    fn display_invalid_request() {
        let err = TitenError::InvalidRequest("missing field".into());
        assert!(err.to_string().contains("Invalid request"));
    }

    #[test]
    fn display_config_error() {
        let err = TitenError::ConfigError("missing api key".into());
        assert!(err.to_string().contains("Configuration error"));
    }

    // ─── From conversions ─────────────────────────────────

    #[test]
    fn from_sqlx_error() {
        let sqlx_err = sqlx::Error::ColumnNotFound("nonexistent".into());
        let titen_err: TitenError = sqlx_err.into();
        let msg = titen_err.to_string();
        assert!(msg.contains("Database error"));
    }

    #[test]
    fn from_reqwest_error() {
        // Verify the From impl is wired correctly via the type system.
        // Constructing a real reqwest::Error synchronously requires a request,
        // so we verify compilation + a round-trip through a helper.
        fn convert(err: reqwest::Error) -> TitenError {
            TitenError::from(err)
        }
        let _converter: fn(reqwest::Error) -> TitenError = convert;
    }

    // ─── Result type ───────────────────────────────────────

    #[test]
    fn result_type_works() {
        fn returns_ok() -> Result<i32> {
            Ok(42)
        }
        fn returns_err() -> Result<i32> {
            Err(TitenError::AccountNotFound("x".into()))
        }
        assert_eq!(returns_ok().unwrap(), 42);
        assert!(returns_err().is_err());
    }
}
