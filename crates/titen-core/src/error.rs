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

pub type Result<T> = std::result::Result<T, TitenError>;
