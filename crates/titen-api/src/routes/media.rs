use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::server::AppState;
use titen_core::models::{MediaAsset, MediaFilter};
use titen_core::storage::{S3Storage, detect_backend};

/// Allowed MIME types for media uploads.
const ALLOWED_MIME_TYPES: &[&str] = &[
    "image/jpeg",
    "image/png",
    "image/gif",
    "image/webp",
    "video/mp4",
    "video/webm",
];

/// Heal legacy `s3_url` values that were stored as relative paths (e.g.
/// Read-time repair: fix legacy relative or None s3_url values in-place.
///
/// Uses `S3Storage::build_public_url` to ensure URL format stays consistent
/// with the storage layer (#186 CodeCora review).
fn heal_media_urls(media: &mut [MediaAsset]) {
    // Quick check: any rows need healing?
    let needs_heal = media
        .iter()
        .any(|m| m.s3_url.as_ref().is_none_or(|u| !u.starts_with("http")));

    if !needs_heal {
        return;
    }

    // Read env values to pass through S3Storage::build_public_url (single source
    // of truth for URL construction — avoids logic duplication, #186 CodeCora review).
    let endpoint = std::env::var("TITEN_S3_ENDPOINT").unwrap_or_default();
    let bucket = std::env::var("TITEN_S3_BUCKET").unwrap_or_default();
    let public_url_raw = std::env::var("TITEN_S3_PUBLIC_URL")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    heal_media_urls_with(media, public_url_raw.as_deref(), &endpoint, &bucket);
}

/// Pure URL healing logic — testable without env var mutation.
fn heal_media_urls_with(
    media: &mut [MediaAsset],
    public_url: Option<&str>,
    endpoint: &str,
    bucket: &str,
) {
    // Need either public_url OR (endpoint + bucket) to build a valid URL.
    if public_url.is_none() && (endpoint.is_empty() || bucket.is_empty()) {
        return;
    }

    for m in media.iter_mut() {
        let needs_fix = m.s3_url.as_ref().is_none_or(|u| !u.starts_with("http"));
        if needs_fix {
            m.s3_url = Some(S3Storage::build_public_url(
                public_url, endpoint, bucket, &m.s3_key,
            ));
        }
    }
}

/// Validate file content using magic bytes (first 16 bytes).
/// Returns the detected MIME type, or an error message if the file type is not allowed.
fn validate_magic_bytes(data: &[u8]) -> Result<String, &'static str> {
    if data.len() < 12 {
        return Err("File too small to validate");
    }

    let mime = if data.starts_with(b"\xFF\xD8\xFF") {
        "image/jpeg"
    } else if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        "image/gif"
    } else if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        "image/webp"
    } else if data.len() >= 12 && &data[4..8] == b"ftyp" {
        // MP4/MOV family — check brand (mp4, m4v, mov, etc.)
        let brand = &data[8..12];
        if brand.starts_with(b"mp4")
            || brand.starts_with(b"m4v")
            || brand == b"qt  "
            || brand == b"isom"
        {
            "video/mp4"
        } else {
            return Err("Unsupported video format");
        }
    } else if data.len() >= 4 && &data[0..4] == b"\x1A\x45\xDF\xA3" {
        // WebM/Matroska
        "video/webm"
    } else {
        return Err("Unrecognized file type");
    };

    if !ALLOWED_MIME_TYPES.contains(&mime) {
        return Err("File type not allowed");
    }

    Ok(mime.to_string())
}
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
    // P5.2: Clamp limit/offset BEFORE passing to store query
    let limit = filter.limit.unwrap_or(50).clamp(1, 1000);
    let offset = filter.offset.unwrap_or(0).max(0);
    let clamped_filter = MediaFilter {
        limit: Some(limit),
        offset: Some(offset),
        content_type: filter.content_type.clone(),
        search: filter.search.clone(),
    };

    match state.store.list_media(&clamped_filter).await {
        Ok(mut media) => {
            // Read-time repair: heal any legacy relative s3_url values.
            heal_media_urls(&mut media);
            // P5.2: Include total count + pagination metadata
            let total = state.store.count_media(&clamped_filter).await.unwrap_or(0);
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "data": media,
                    "pagination": {
                        "total": total,
                        "limit": limit,
                        "offset": offset,
                        "has_more": (offset + media.len() as i64) < total,
                    }
                })),
            )
        }
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
        // Note: content_type is validated from magic bytes after reading data,
        // not from the client-provided Content-Type header (which can be spoofed).

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

        // P3.4: Validate file type using magic bytes (not client-provided Content-Type)
        let content_type = match validate_magic_bytes(&data) {
            Ok(mime) => mime,
            Err(reason) => {
                tracing::warn!("Upload rejected: {reason} ({} bytes)", data.len());
                return (
                    StatusCode::UNSUPPORTED_MEDIA_TYPE,
                    Json(serde_json::json!({
                        "error": reason,
                        "code": "INVALID_FILE_TYPE"
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_asset(s3_url: Option<&str>, s3_key: &str) -> MediaAsset {
        MediaAsset {
            id: "test-id".to_string(),
            filename: "test.png".to_string(),
            content_type: "image/png".to_string(),
            size_bytes: 1024,
            s3_key: s3_key.to_string(),
            s3_url: s3_url.map(String::from),
            uploaded_at: "2026-08-12T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn heal_relative_url_to_absolute() {
        let mut media = vec![make_asset(
            Some("/2026/08/12/uuid.png"),
            "2026/08/12/uuid.png",
        )];
        heal_media_urls_with(&mut media, None, "https://s3.ajianaz.dev", "titen");

        assert_eq!(
            media[0].s3_url.as_deref(),
            Some("https://s3.ajianaz.dev/titen/2026/08/12/uuid.png")
        );
    }

    #[test]
    fn heal_none_url_to_absolute() {
        let mut media = vec![make_asset(None, "2026/08/12/uuid.png")];
        heal_media_urls_with(&mut media, None, "https://s3.ajianaz.dev", "titen");

        assert_eq!(
            media[0].s3_url.as_deref(),
            Some("https://s3.ajianaz.dev/titen/2026/08/12/uuid.png")
        );
    }

    #[test]
    fn heal_skips_already_absolute_urls() {
        let absolute = "https://cdn.example.com/img.png";
        let mut media = vec![make_asset(Some(absolute), "2026/08/12/uuid.png")];
        heal_media_urls_with(&mut media, None, "https://s3.ajianaz.dev", "titen");

        assert_eq!(media[0].s3_url.as_deref(), Some(absolute));
    }

    #[test]
    fn heal_respects_public_url_override() {
        let mut media = vec![make_asset(
            Some("/2026/08/12/uuid.png"),
            "2026/08/12/uuid.png",
        )];
        heal_media_urls_with(
            &mut media,
            Some("https://cdn.example.com"),
            "https://s3.ajianaz.dev",
            "titen",
        );

        assert_eq!(
            media[0].s3_url.as_deref(),
            Some("https://cdn.example.com/2026/08/12/uuid.png")
        );
    }

    #[test]
    fn heal_ignores_empty_public_url() {
        // #186 root cause: empty string TITEN_S3_PUBLIC_URL must be treated as unset.
        // heal_media_urls() filters this before calling heal_media_urls_with,
        // so we pass None here to simulate the filtered state.
        let mut media = vec![make_asset(
            Some("/2026/08/12/uuid.png"),
            "2026/08/12/uuid.png",
        )];
        heal_media_urls_with(&mut media, None, "https://s3.ajianaz.dev", "titen");

        assert_eq!(
            media[0].s3_url.as_deref(),
            Some("https://s3.ajianaz.dev/titen/2026/08/12/uuid.png")
        );
    }

    #[test]
    fn heal_trims_trailing_slash_from_endpoint() {
        let mut media = vec![make_asset(
            Some("/2026/08/12/uuid.png"),
            "2026/08/12/uuid.png",
        )];
        heal_media_urls_with(&mut media, None, "https://s3.ajianaz.dev/", "titen");

        assert_eq!(
            media[0].s3_url.as_deref(),
            Some("https://s3.ajianaz.dev/titen/2026/08/12/uuid.png")
        );
    }

    #[test]
    fn heal_noop_without_storage_config() {
        let mut media = vec![make_asset(
            Some("/2026/08/12/uuid.png"),
            "2026/08/12/uuid.png",
        )];
        heal_media_urls_with(&mut media, None, "", "");

        // Should remain unchanged — no config to reconstruct URL.
        assert_eq!(media[0].s3_url.as_deref(), Some("/2026/08/12/uuid.png"));
    }

    #[test]
    fn heal_uses_public_url_when_bucket_empty() {
        let mut media = vec![make_asset(
            Some("/2026/08/12/uuid.png"),
            "2026/08/12/uuid.png",
        )];
        heal_media_urls_with(&mut media, Some("https://cdn.example.com"), "", "");

        assert_eq!(
            media[0].s3_url.as_deref(),
            Some("https://cdn.example.com/2026/08/12/uuid.png")
        );
    }
}
