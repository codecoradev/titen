use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::server::AppState;
use titen_core::models::MediaFilter;
use titen_core::storage::{S3Storage, detect_backend};

#[utoipa::path(
    get,
    path = "/api/media",
    tag = "media",
    params(("filter" = Option<MediaFilter>, Query, description = "Media filter")),
    responses(
        (status = 200, description = "List of media assets", body = serde_json::Value),
    ),
    security(("api_key" = [])),
)]
pub async fn list_media(
    State(state): State<AppState>,
    Query(filter): Query<MediaFilter>,
) -> (StatusCode, Json<serde_json::Value>) {
    match state.store.list_media(&filter).await {
        Ok(media) => (StatusCode::OK, Json(serde_json::json!({ "data": media }))),
        Err(e) => {
            tracing::error!("Failed to list media: {e:?}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to retrieve media list",
                    "code": "LIST_FAILED"
                })),
            )
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/media",
    tag = "media",
    responses(
        (status = 201, description = "Media uploaded", body = serde_json::Value),
        (status = 400, description = "Bad request", body = serde_json::Value),
        (status = 503, description = "S3 not configured", body = serde_json::Value),
    ),
    security(("api_key" = [])),
)]
pub async fn upload_media(
    State(state): State<AppState>,
    mut multipart: axum::extract::Multipart,
) -> (StatusCode, Json<serde_json::Value>) {
    let storage = match detect_backend() {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Storage backend not configured: {e}");
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "error": "Storage backend not configured",
                    "code": "STORAGE_NOT_CONFIGURED"
                })),
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
                tracing::warn!("Failed to read uploaded file: {e}");
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "Failed to read uploaded file",
                        "code": "READ_FAILED"
                    })),
                );
            }
        };

        let s3_key = S3Storage::build_key(&filename);
        let media_url = match storage.upload(&s3_key, &data, &content_type).await {
            Ok(url) => url,
            Err(e) => {
                tracing::error!("Storage upload failed: {e:?}");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": "Failed to upload file to storage",
                        "code": "UPLOAD_FAILED"
                    })),
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
                Some(&media_url),
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
                tracing::error!("Failed to store media record: {e:?}");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": "Failed to store media record",
                        "code": "DB_FAILED"
                    })),
                );
            }
        }
    }

    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({ "error": "No file provided", "code": "NO_FILE" })),
    )
}

#[utoipa::path(
    delete,
    path = "/api/media/{id}",
    tag = "media",
    params(("id" = String, Path, description = "Media asset ID")),
    responses(
        (status = 200, description = "Media deleted", body = serde_json::Value),
        (status = 404, description = "Not found", body = serde_json::Value),
    ),
    security(("api_key" = [])),
)]
pub async fn delete_media(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Get media record by ID to find storage key
    match state.store.get_media_asset(&id).await {
        Ok(media) => {
            // Try to delete from storage backend (best effort)
            if let Ok(storage) = detect_backend() {
                if let Err(e) = storage.delete(&media.s3_key).await {
                    tracing::warn!("Failed to delete media {}: {e}", media.s3_key);
                }
            }
        }
        Err(e) => {
            tracing::warn!("Media asset {} not found: {e}", id);
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Media asset not found",
                    "code": "NOT_FOUND"
                })),
            );
        }
    }

    match state.store.delete_media(&id).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({ "data": null }))),
        Err(e) => {
            tracing::error!("Failed to delete media {}: {e:?}", id);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to delete media asset",
                    "code": "DELETE_FAILED"
                })),
            )
        }
    }
}
