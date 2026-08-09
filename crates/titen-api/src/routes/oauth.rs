use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::server::{AppState, error_response};

/// OAuth exchange request.
///
/// Supports two modes:
/// - **Server-side (preferred)**: `{ code, redirect_uri }` — app_id and app_secret
///   are read from the `app_settings` table (encrypted).
/// - **Legacy (backward compat)**: `{ code, app_id, app_secret, redirect_uri }` —
///   for older FE builds and CLI/MCP clients that pass credentials directly.
#[derive(Deserialize)]
pub struct OAuthExchangeRequest {
    pub code: String,
    pub redirect_uri: String,
    /// Optional — if omitted, read from app_settings DB table.
    pub app_id: Option<String>,
    /// Optional — if omitted, read from app_settings DB table (decrypted).
    pub app_secret: Option<String>,
}

/// Exchange an OAuth authorization code for a full account.
///
/// Flow:
/// 1. Resolve app_id + app_secret (from request or DB)
/// 2. Exchange code → short-lived token + user_id
/// 3. Exchange short-lived → long-lived token
/// 4. Resolve username from /me
/// 5. Create account in DB
pub async fn oauth_exchange(
    State(state): State<AppState>,
    Json(input): Json<OAuthExchangeRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Resolve app_id: prefer request field, fall back to DB
    let app_id = match input.app_id.as_deref() {
        Some(id) if !id.is_empty() => id.to_string(),
        _ => match state.store.get_app_settings().await {
            Ok(s) => match s.threads_app_id {
                Some(id) if !id.is_empty() => id,
                _ => {
                    warn!(target: "titen::oauth", "OAUTH_EXCHANGE_FAIL no app_id in request or DB");
                    let (status, body) = error_response(
                        StatusCode::BAD_REQUEST,
                        "MISSING_APP_ID",
                        "No Threads App ID configured. Set it in Settings or provide app_id in the request.",
                    );
                    return (
                        status,
                        Json(serde_json::json!({ "error": body.error, "code": body.code })),
                    );
                }
            },
            Err(e) => {
                error!(target: "titen::oauth", "OAUTH_EXCHANGE_FAIL db error reading settings: {}", e);
                let (status, body) = error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "SETTINGS_READ_FAILED",
                    &e.to_string(),
                );
                return (
                    status,
                    Json(serde_json::json!({ "error": body.error, "code": body.code })),
                );
            }
        },
    };

    // Resolve app_secret: prefer request field, fall back to DB (decrypted)
    let app_secret = match input.app_secret.as_deref() {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => match state.store.get_threads_app_secret().await {
            Ok(Some(s)) => s,
            Ok(None) => {
                warn!(target: "titen::oauth", "OAUTH_EXCHANGE_FAIL no app_secret in request or DB");
                let (status, body) = error_response(
                    StatusCode::BAD_REQUEST,
                    "MISSING_APP_SECRET",
                    "No Threads App Secret configured. Set it in Settings or provide app_secret in the request.",
                );
                return (
                    status,
                    Json(serde_json::json!({ "error": body.error, "code": body.code })),
                );
            }
            Err(e) => {
                error!(target: "titen::oauth", "OAUTH_EXCHANGE_FAIL db error reading secret: {}", e);
                let (status, body) = error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "SECRET_DECRYPT_FAILED",
                    &e.to_string(),
                );
                return (
                    status,
                    Json(serde_json::json!({ "error": body.error, "code": body.code })),
                );
            }
        },
    };

    info!(target: "titen::oauth", "OAUTH_EXCHANGE_START code_len={} app_id_len={} redirect_uri={}", input.code.len(), app_id.len(), input.redirect_uri);

    // Step 1: code → short-lived token + user_id
    info!(target: "titen::oauth", "OAUTH_STEP1 exchange code for short-lived token...");
    let (short_token, user_id) = match state
        .threads_client
        .exchange_code_for_token(&input.code, &app_id, &app_secret, &input.redirect_uri)
        .await
    {
        Ok(result) => {
            info!(target: "titen::oauth", "OAUTH_STEP1_OK user_id={}", result.1);
            result
        }
        Err(e) => {
            warn!(target: "titen::oauth", "OAUTH_STEP1_FAIL {}", e);
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
    info!(target: "titen::oauth", "OAUTH_STEP2 exchange for long-lived token...");
    let (long_token, expires_in) = match state
        .threads_client
        .exchange_long_lived_token(&short_token, &app_secret)
        .await
    {
        Ok(result) => {
            info!(target: "titen::oauth", "OAUTH_STEP2_OK expires_in={}s", result.1);
            result
        }
        Err(e) => {
            warn!(target: "titen::oauth", "OAUTH_STEP2_FAIL {}", e);
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
    info!(target: "titen::oauth", "OAUTH_STEP3 resolve account...");
    let (_resolved_id, username) = match state.threads_client.resolve_account(&long_token).await {
        Ok(result) => {
            info!(target: "titen::oauth", "OAUTH_STEP3_OK username={}", result.1);
            result
        }
        Err(e) => {
            warn!(target: "titen::oauth", "OAUTH_STEP3_FAIL {}", e);
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
    info!(target: "titen::oauth", "OAUTH_STEP4 create account in DB...");
    let id = Uuid::now_v7().to_string();
    let expires_at = (chrono::Utc::now() + chrono::Duration::seconds(expires_in)).to_rfc3339();

    let create_input = titen_core::models::CreateAccount {
        user_id: Some(user_id),
        username: Some(username),
        access_token: long_token,
        expires_at,
        app_id: Some(app_id.clone()),
        app_secret: None,
    };

    match state.store.create_account(&id, &create_input).await {
        Ok(account) => {
            info!(target: "titen::oauth", "OAUTH_EXCHANGE_SUCCESS account_id={} username={}", account.id, account.username);
            (
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
            )
        }
        Err(e) => {
            error!(target: "titen::oauth", "OAUTH_STEP4_FAIL db error: {}", e);
            let (status, body) =
                error_response(StatusCode::CONFLICT, "CREATE_FAILED", &e.to_string());
            (
                status,
                Json(serde_json::json!({ "error": body.error, "code": body.code })),
            )
        }
    }
}
