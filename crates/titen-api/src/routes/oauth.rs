use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;
use uuid::Uuid;

use crate::server::{AppState, error_response};

#[derive(Deserialize)]
pub struct OAuthExchangeRequest {
    pub code: String,
    pub app_id: String,
    pub app_secret: String,
    pub redirect_uri: String,
}

/// Exchange an OAuth authorization code for a full account.
///
/// Flow:
/// 1. Exchange code → short-lived token + user_id
/// 2. Exchange short-lived → long-lived token
/// 3. Resolve username from /me
/// 4. Create account in DB
pub async fn oauth_exchange(
    State(state): State<AppState>,
    Json(input): Json<OAuthExchangeRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Step 1: code → short-lived token + user_id
    let (short_token, user_id) = match state
        .threads_client
        .exchange_code_for_token(
            &input.code,
            &input.app_id,
            &input.app_secret,
            &input.redirect_uri,
        )
        .await
    {
        Ok(result) => result,
        Err(e) => {
            let (status, body) = error_response(
                StatusCode::BAD_REQUEST,
                "OAUTH_EXCHANGE_FAILED",
                &format!("Failed to exchange code: {e}"),
            );
            return (
                status,
                Json(serde_json::json!({ "error": body.error, "code": body.code })),
            );
        }
    };

    // Step 2: short-lived → long-lived
    let (long_token, expires_in) = match state
        .threads_client
        .exchange_long_lived_token(&short_token, &input.app_secret)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            let (status, body) = error_response(
                StatusCode::BAD_REQUEST,
                "TOKEN_EXCHANGE_FAILED",
                &format!("Failed to get long-lived token: {e}"),
            );
            return (
                status,
                Json(serde_json::json!({ "error": body.error, "code": body.code })),
            );
        }
    };

    // Step 3: resolve username
    let (_resolved_id, username) = match state.threads_client.resolve_account(&long_token).await {
        Ok(result) => result,
        Err(e) => {
            let (status, body) = error_response(
                StatusCode::BAD_REQUEST,
                "RESOLVE_FAILED",
                &format!("Failed to resolve account: {e}"),
            );
            return (
                status,
                Json(serde_json::json!({ "error": body.error, "code": body.code })),
            );
        }
    };

    // Step 4: create account
    let id = Uuid::now_v7().to_string();
    let expires_at = (chrono::Utc::now() + chrono::Duration::seconds(expires_in)).to_rfc3339();

    let create_input = titen_core::models::CreateAccount {
        user_id: Some(user_id),
        username: Some(username),
        access_token: long_token,
        expires_at,
        app_id: Some(input.app_id),
        app_secret: None,
    };

    match state.store.create_account(&id, &create_input).await {
        Ok(account) => (
            StatusCode::CREATED,
            Json(serde_json::json!({
                "data": {
                    "id": account.id,
                    "username": account.username,
                    "user_id": account.user_id,
                    "is_active": account.is_active,
                    "expires_at": account.expires_at,
                    "token_status": account.token_status(),
                    "created_at": account.created_at,
                }
            })),
        ),
        Err(e) => {
            let (status, body) =
                error_response(StatusCode::CONFLICT, "CREATE_FAILED", &e.to_string());
            (
                status,
                Json(serde_json::json!({ "error": body.error, "code": body.code })),
            )
        }
    }
}
