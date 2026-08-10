//! Error sanitization — newtype wrapper around `TitenError` that implements
//! `IntoResponse` without violating orphan rules.
//!
//! Prevents internal error details (filesystem paths, connection strings,
//! SQL fragments) from leaking to API consumers. Sensitive variants are
//! mapped to generic messages; the full error is logged server-side via
//! `tracing::error!`.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use titen_core::TitenError;

/// Wrapper around `TitenError` that implements `IntoResponse`.
/// Use `ApiError(err).into_response()` or return `ApiError(err)` from handlers.
pub struct ApiError(pub TitenError);

#[derive(serde::Serialize)]
struct ApiErrorResponse {
    error: &'static str,
    message: String,
}

impl From<TitenError> for ApiError {
    fn from(err: TitenError) -> Self {
        ApiError(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_type, safe_message) = match &self.0 {
            // 404 — Not Found variants (safe to expose the ID)
            TitenError::AccountNotFound(id) => (
                StatusCode::NOT_FOUND,
                "not_found",
                format!("Account not found: {id}"),
            ),
            TitenError::PostNotFound(id) => (
                StatusCode::NOT_FOUND,
                "not_found",
                format!("Post not found: {id}"),
            ),
            TitenError::ScheduleNotFound(id) => (
                StatusCode::NOT_FOUND,
                "not_found",
                format!("Schedule not found: {id}"),
            ),
            TitenError::MediaNotFound(id) => (
                StatusCode::NOT_FOUND,
                "not_found",
                format!("Media not found: {id}"),
            ),
            TitenError::CommentNotFound(id) => (
                StatusCode::NOT_FOUND,
                "not_found",
                format!("Comment not found: {id}"),
            ),

            // 409 — Conflict
            TitenError::AccountAlreadyExists(name) => (
                StatusCode::CONFLICT,
                "conflict",
                format!("Account already exists: {name}"),
            ),

            // 400 — Bad Request (safe to expose)
            TitenError::InvalidRequest(msg) => {
                (StatusCode::BAD_REQUEST, "invalid_request", msg.clone())
            }

            // 401 — Token issues (safe to expose account name)
            TitenError::TokenExpired(account) => (
                StatusCode::UNAUTHORIZED,
                "token_expired",
                format!("Token expired for account: {account}"),
            ),
            TitenError::TokenRefreshFailed(account) => (
                StatusCode::UNAUTHORIZED,
                "token_refresh_failed",
                format!("Token refresh failed for account: {account}"),
            ),

            // 429 — Rate limited (safe to expose the limits)
            TitenError::RateLimitExceeded {
                action,
                current,
                limit,
            } => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                format!("Rate limit exceeded: {action} ({current}/{limit} in 24h)"),
            ),

            // ── SENSITIVE: sanitize internal details ──────────────
            TitenError::DatabaseError(_) => {
                tracing::error!(error = %self.0, "Database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "An internal database error occurred".to_string(),
                )
            }
            TitenError::ThreadsApiError(_) => {
                tracing::error!(error = %self.0, "Threads API error");
                (
                    StatusCode::BAD_GATEWAY,
                    "upstream_error",
                    "Threads API request failed".to_string(),
                )
            }
            TitenError::StorageError(_) => {
                tracing::error!(error = %self.0, "Storage error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "Storage operation failed".to_string(),
                )
            }
            TitenError::SentimentError(_) => {
                tracing::error!(error = %self.0, "Sentiment error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "Sentiment analysis failed".to_string(),
                )
            }
            TitenError::ConfigError(_) => {
                tracing::error!(error = %self.0, "Configuration error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "Server configuration error".to_string(),
                )
            }
        };

        tracing::debug!(%error_type, %status, "Request error response");

        (
            status,
            Json(ApiErrorResponse {
                error: error_type,
                message: safe_message,
            }),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_error_returns_500() {
        let err = ApiError(TitenError::DatabaseError(
            "no such table: users /db/data.sqlite".into(),
        ));
        let response = err.into_response();
        assert_eq!(response.status(), 500);
    }

    #[test]
    fn threads_api_error_returns_502() {
        let err = ApiError(TitenError::ThreadsApiError(
            "token=abc123 expired at /secret/path".into(),
        ));
        let response = err.into_response();
        assert_eq!(response.status(), 502);
    }

    #[test]
    fn storage_error_returns_500() {
        let err = ApiError(TitenError::StorageError(
            "Failed to connect to https://s3.amazonaws.com/bucket".into(),
        ));
        let response = err.into_response();
        assert_eq!(response.status(), 500);
    }

    #[test]
    fn account_not_found_returns_404() {
        let err = ApiError(TitenError::AccountNotFound("user123".into()));
        let response = err.into_response();
        assert_eq!(response.status(), 404);
    }

    #[test]
    fn invalid_request_returns_400() {
        let err = ApiError(TitenError::InvalidRequest("missing field: username".into()));
        let response = err.into_response();
        assert_eq!(response.status(), 400);
    }

    #[test]
    fn rate_limit_returns_429() {
        let err = ApiError(TitenError::RateLimitExceeded {
            action: "post".into(),
            current: 250,
            limit: 250,
        });
        let response = err.into_response();
        assert_eq!(response.status(), 429);
    }

    #[test]
    fn account_already_exists_returns_409() {
        let err = ApiError(TitenError::AccountAlreadyExists("alice".into()));
        let response = err.into_response();
        assert_eq!(response.status(), 409);
    }

    #[test]
    fn token_expired_returns_401() {
        let err = ApiError(TitenError::TokenExpired("acc1".into()));
        let response = err.into_response();
        assert_eq!(response.status(), 401);
    }

    #[test]
    fn config_error_returns_500() {
        let err = ApiError(TitenError::ConfigError("missing TITEN_DB_PATH".into()));
        let response = err.into_response();
        assert_eq!(response.status(), 500);
    }

    #[test]
    fn sentiment_error_returns_500() {
        let err = ApiError(TitenError::SentimentError(
            "model not found at /opt/model.bin".into(),
        ));
        let response = err.into_response();
        assert_eq!(response.status(), 500);
    }

    #[test]
    fn from_titen_error_conversion() {
        let core_err = TitenError::InvalidRequest("test".into());
        let api_err = ApiError::from(core_err);
        let response = api_err.into_response();
        assert_eq!(response.status(), 400);
    }
}
