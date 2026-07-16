use axum::{
    Json,
    extract::{Path, State},
};
use uuid::Uuid;

use crate::server::AppState;
use titen_core::models::*;

pub async fn list_media(State(state): State<AppState>) -> Json<serde_json::Value> {
    match state.store.list_media().await {
        Ok(media) => Json(serde_json::json!({ "data": media })),
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "LIST_FAILED" })),
    }
}

pub async fn upload_media(State(state): State<AppState>) -> Json<serde_json::Value> {
    // TODO: implement multipart upload + S3 storage
    Json(serde_json::json!({ "message": "Media upload not yet implemented" }))
}

pub async fn delete_media(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match state.store.delete_media(&id).await {
        Ok(()) => Json(serde_json::json!({ "data": null })),
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "DELETE_FAILED" })),
    }
}
