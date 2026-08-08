use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::server::AppState;
use titen_core::models::MediaFilter;
use titen_core::storage::{S3Storage, Storage};

pub async fn list_media(
    State(state): State<AppState>,
    Query(filter): Query<MediaFilter>,
) -> Json<serde_json::Value> {
    match state.store.list_media(&filter).await {
        Ok(media) => Json(serde_json::json!({ "data": media })),
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "LIST_FAILED" })),
    }
}

pub async fn upload_media(
    State(state): State<AppState>,
    mut multipart: axum::extract::Multipart,
) -> (StatusCode, Json<serde_json::Value>) {
    let s3 = match S3Storage::from_env() {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({ "error": e.to_string(), "code": "S3_NOT_CONFIGURED" })),
            );
        }
    };

    if let Ok(Some(field)) = multipart.next_field().await {
        let filename = field.file_name().unwrap_or("unknown").to_string();
        let content_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();

        // Read bytes
        let data = match field.bytes().await {
            Ok(d) => d.to_vec(),
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(
                        serde_json::json!({ "error": format!("Failed to read file: {e}"), "code": "READ_FAILED" }),
                    ),
                );
            }
        };

        let s3_key = S3Storage::build_key(&filename);
        let s3_url = match s3.upload(&s3_key, &data, &content_type).await {
            Ok(url) => url,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e.to_string(), "code": "UPLOAD_FAILED" })),
                );
            }
        };

        let id = Uuid::now_v7().to_string();
        match state
            .store
            .create_media_asset(
                &id,
                &filename,
                &content_type,
                data.len() as i64,
                &s3_key,
                Some(&s3_url),
            )
            .await
        {
            Ok(asset) => {
                return (
                    StatusCode::CREATED,
                    Json(serde_json::json!({ "data": asset })),
                );
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e.to_string(), "code": "DB_FAILED" })),
                );
            }
        }
    }

    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({ "error": "No file provided", "code": "NO_FILE" })),
    )
}

pub async fn delete_media(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    // Get media record to find S3 key
    let media_list = match state.store.list_media(&MediaFilter::default()).await {
        Ok(m) => m,
        Err(e) => {
            return Json(serde_json::json!({ "error": e.to_string(), "code": "LIST_FAILED" }));
        }
    };

    if let Some(media) = media_list.iter().find(|m| m.id == id) {
        // Try to delete from S3 (best effort)
        if let Ok(s3) = S3Storage::from_env() {
            if let Err(e) = s3.delete(&media.s3_key).await {
                tracing::warn!("Failed to delete S3 object {}: {e}", media.s3_key);
            }
        }
    }

    match state.store.delete_media(&id).await {
        Ok(()) => Json(serde_json::json!({ "data": null })),
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "DELETE_FAILED" })),
    }
}
