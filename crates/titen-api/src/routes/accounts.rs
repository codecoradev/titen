use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::server::{AppState, error_response};
use titen_core::models::*;

/// Return a safe account JSON (no access_token or app_secret).
fn safe_account_json(account: &titen_core::models::Account) -> serde_json::Value {
    serde_json::json!({
        "id": account.id,
        "username": account.username,
        "user_id": account.user_id,
        "is_active": account.is_active,
        "expires_at": account.expires_at,
        "token_status": account.token_status(),
        "created_at": account.created_at,
        "updated_at": account.updated_at,
    })
}

/// List all accounts (safe view — no tokens).
#[utoipa::path(
    get,
    path = "/api/accounts",
    tag = "accounts",
    responses(
        (status = 200, description = "List of accounts", body = serde_json::Value),
    ),
    security(("api_key" = [])),
)]
pub async fn list_accounts(State(state): State<AppState>) -> Json<serde_json::Value> {
    match state.store.list_accounts().await {
        Ok(accounts) => {
            let data: Vec<serde_json::Value> = accounts.iter().map(safe_account_json).collect();
            Json(serde_json::json!({ "data": data }))
        }
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "LIST_FAILED" })),
    }
}

#[utoipa::path(
    post,
    path = "/api/accounts",
    tag = "accounts",
    request_body = CreateAccount,
    responses(
        (status = 201, description = "Account created", body = serde_json::Value),
        (status = 400, description = "Bad request", body = serde_json::Value),
        (status = 409, description = "Conflict", body = serde_json::Value),
    ),
    security(("api_key" = [])),
)]
pub async fn create_account(
    State(state): State<AppState>,
    Json(mut input): Json<CreateAccount>,
) -> (StatusCode, Json<serde_json::Value>) {
    let id = Uuid::now_v7().to_string();

    // Auto-resolve username + user_id from /me if not provided
    if input.username.is_none() || input.user_id.is_none() {
        match state
            .threads_client
            .resolve_account(&input.access_token)
            .await
        {
            Ok((user_id, username)) => {
                if input.username.is_none() {
                    input.username = Some(username);
                }
                if input.user_id.is_none() {
                    input.user_id = Some(user_id);
                }
            }
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": format!("Failed to resolve account: {e}"),
                        "code": "RESOLVE_FAILED"
                    })),
                );
            }
        }
    }

    // Auto-exchange short-lived → long-lived token if app_secret provided
    if let Some(ref app_secret) = input.app_secret {
        if !app_secret.is_empty() {
            match state
                .threads_client
                .exchange_long_lived_token(&input.access_token, app_secret)
                .await
            {
                Ok((long_token, expires_in)) => {
                    input.access_token = long_token;
                    let expires_at =
                        (chrono::Utc::now() + chrono::Duration::seconds(expires_in)).to_rfc3339();
                    input.expires_at = expires_at;
                }
                Err(e) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({
                            "error": format!("Token exchange failed: {e}"),
                            "code": "EXCHANGE_FAILED"
                        })),
                    );
                }
            }
        }
    }

    match state.store.create_account(&id, &input).await {
        Ok(account) => (
            StatusCode::CREATED,
            Json(serde_json::json!({ "data": safe_account_json(&account) })),
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

#[utoipa::path(
    put,
    path = "/api/accounts/{id}",
    tag = "accounts",
    params(("id" = String, Path, description = "Account ID")),
    request_body = UpdateAccount,
    responses(
        (status = 200, description = "Account updated", body = serde_json::Value),
        (status = 404, description = "Not found", body = serde_json::Value),
    ),
    security(("api_key" = [])),
)]
pub async fn update_account(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<UpdateAccount>,
) -> Json<serde_json::Value> {
    match state.store.update_account(&id, &input).await {
        Ok(account) => Json(serde_json::json!({ "data": safe_account_json(&account) })),
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "UPDATE_FAILED" })),
    }
}

#[utoipa::path(
    delete,
    path = "/api/accounts/{id}",
    tag = "accounts",
    params(("id" = String, Path, description = "Account ID")),
    responses(
        (status = 200, description = "Account deleted", body = serde_json::Value),
        (status = 404, description = "Not found", body = serde_json::Value),
    ),
    security(("api_key" = [])),
)]
pub async fn delete_account(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match state.store.delete_account(&id).await {
        Ok(()) => Json(serde_json::json!({ "data": null })),
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "DELETE_FAILED" })),
    }
}

#[utoipa::path(
    post,
    path = "/api/accounts/{id}/refresh-token",
    tag = "accounts",
    params(("id" = String, Path, description = "Account ID")),
    responses(
        (status = 200, description = "Token refreshed", body = serde_json::Value),
        (status = 404, description = "Not found", body = serde_json::Value),
    ),
    security(("api_key" = [])),
)]
pub async fn refresh_token(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match state.store.get_account(&id).await {
        Ok(account) => match state.threads_client.refresh_token(&account).await {
            Ok(updated) => Json(serde_json::json!({ "data": safe_account_json(&updated) })),
            Err(e) => Json(serde_json::json!({
                "error": e.to_string(),
                "code": "REFRESH_FAILED"
            })),
        },
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "NOT_FOUND" })),
    }
}
