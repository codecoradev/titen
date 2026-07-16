use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::server::{AppState, error_response};
use titen_core::models::*;

pub async fn list_accounts(State(state): State<AppState>) -> Json<serde_json::Value> {
    match state.store.list_accounts().await {
        Ok(accounts) => Json(serde_json::json!({ "data": accounts })),
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "LIST_FAILED" })),
    }
}

pub async fn create_account(
    State(state): State<AppState>,
    Json(input): Json<CreateAccount>,
) -> (StatusCode, Json<serde_json::Value>) {
    let id = Uuid::now_v7().to_string();
    match state.store.create_account(&id, &input).await {
        Ok(account) => (
            StatusCode::CREATED,
            Json(serde_json::json!({ "data": account })),
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

pub async fn update_account(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<UpdateAccount>,
) -> Json<serde_json::Value> {
    match state.store.update_account(&id, &input).await {
        Ok(account) => Json(serde_json::json!({ "data": account })),
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "UPDATE_FAILED" })),
    }
}

pub async fn delete_account(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match state.store.delete_account(&id).await {
        Ok(()) => Json(serde_json::json!({ "data": null })),
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "DELETE_FAILED" })),
    }
}

pub async fn refresh_token(
    State(_state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "message": "Token refresh not yet implemented", "account_id": id }))
}
