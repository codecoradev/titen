use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::server::AppState;
use titen_core::models::*;

pub async fn list_posts(
    State(state): State<AppState>,
    Query(filter): Query<PostFilter>,
) -> Json<serde_json::Value> {
    match state.store.list_posts(&filter).await {
        Ok(posts) => Json(serde_json::json!({ "data": posts })),
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "LIST_FAILED" })),
    }
}

pub async fn get_post(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match state.store.get_post(&id).await {
        Ok(post) => Json(serde_json::json!({ "data": post })),
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "NOT_FOUND" })),
    }
}

pub async fn create_post(
    State(state): State<AppState>,
    Json(input): Json<CreatePost>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Get account for Threads API call
    let account = match state.store.get_account(&input.account_id).await {
        Ok(a) => a,
        Err(e) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e.to_string(), "code": "ACCOUNT_NOT_FOUND" })),
            );
        }
    };

    // Check rate limit
    if let Err(e) = state
        .store
        .check_rate_limit(&input.account_id, "post", 250)
        .await
    {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({ "error": e.to_string(), "code": "RATE_LIMITED" })),
        );
    }

    // Resolve media_ids → S3 URLs (if provided, merge with/replace image_urls for CAROUSEL)
    let mut effective_input = input;
    if let Some(ref media_ids) = effective_input.media_ids {
        if !media_ids.is_empty() {
            let mut resolved_urls = Vec::with_capacity(media_ids.len());
            for mid in media_ids {
                match state.store.get_media_asset(mid).await {
                    Ok(asset) => {
                        if let Some(url) = &asset.s3_url {
                            resolved_urls.push(url.clone());
                        } else {
                            return (
                                StatusCode::BAD_REQUEST,
                                Json(serde_json::json!({
                                    "error": format!("Media asset {} has no S3 URL", mid),
                                    "code": "MEDIA_NO_URL"
                                })),
                            );
                        }
                    }
                    Err(e) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(serde_json::json!({
                                "error": format!("Media asset {} not found: {e}", mid),
                                "code": "MEDIA_NOT_FOUND"
                            })),
                        );
                    }
                }
            }
            // media_ids takes precedence — replaces image_urls
            effective_input.image_urls = Some(resolved_urls);
            if effective_input.media_type.is_none() {
                effective_input.media_type = Some("CAROUSEL".to_string());
            }
        }
    }

    // Validate CAROUSEL before the match (early return for HTTP error)
    if effective_input.media_type.as_deref() == Some("CAROUSEL") {
        let urls = match effective_input
            .image_urls
            .as_ref()
            .filter(|v| !v.is_empty())
        {
            Some(u) => u,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "image_urls (2-20 URLs) is required for CAROUSEL posts",
                        "code": "MISSING_IMAGE_URLS"
                    })),
                );
            }
        };
        if urls.len() < 2 || urls.len() > 20 {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("CAROUSEL requires 2-20 image_urls, got {}", urls.len()),
                    "code": "INVALID_CAROUSEL_COUNT"
                })),
            );
        }
    }

    // Publish via Threads API
    let caption = effective_input.caption.as_deref().unwrap_or("");
    let threads_post_id = match effective_input.media_type.as_deref().unwrap_or("TEXT") {
        "TEXT" => state.threads_client.publish_text(&account, caption).await,
        "IMAGE" => {
            let url = effective_input.image_url.as_deref().unwrap_or("");
            if url.is_empty() {
                Err(titen_core::TitenError::InvalidRequest(
                    "image_url is required for IMAGE posts".to_string(),
                ))
            } else {
                state
                    .threads_client
                    .publish_image(
                        &account,
                        Some(caption),
                        url,
                        effective_input.alt_text.as_deref(),
                    )
                    .await
            }
        }
        "VIDEO" => {
            let url = effective_input.video_url.as_deref().unwrap_or("");
            if url.is_empty() {
                Err(titen_core::TitenError::InvalidRequest(
                    "video_url is required for VIDEO posts".to_string(),
                ))
            } else {
                state
                    .threads_client
                    .publish_video(&account, Some(caption), url)
                    .await
            }
        }
        "CAROUSEL" => {
            // Validation already done above — safe to unwrap
            let urls = effective_input.image_urls.as_ref().unwrap();
            let mut children_ids = Vec::with_capacity(urls.len());
            let mut children_failed = None;
            for url in urls {
                match state
                    .threads_client
                    .create_carousel_item(&account, "IMAGE", Some(url.as_str()), None, None)
                    .await
                {
                    Ok(id) => children_ids.push(id),
                    Err(e) => {
                        tracing::error!(
                            "Partial carousel failure after {n} children. \
                             Orphaned children IDs (manual cleanup needed): {children_ids:?}",
                            n = children_ids.len()
                        );
                        children_failed = Some(e.to_string());
                        break;
                    }
                }
            }
            match children_failed {
                Some(e) => Err(titen_core::TitenError::InvalidRequest(format!(
                    "Failed to create carousel item: {e}"
                ))),
                None => {
                    state
                        .threads_client
                        .publish_carousel(&account, Some(caption), &children_ids)
                        .await
                }
            }
        }
        media => Err(titen_core::TitenError::InvalidRequest(format!(
            "Unsupported media type: {media}"
        ))),
    };

    match threads_post_id {
        Ok(post_id) => {
            // Track rate
            if let Err(e) = state
                .store
                .track_rate(&effective_input.account_id, "post")
                .await
            {
                tracing::warn!("Failed to track rate for post: {e}");
            }

            // Create post record
            let db_id = Uuid::now_v7().to_string();
            match state.store.create_post(&db_id, &effective_input).await {
                Ok(post) => (
                    StatusCode::CREATED,
                    Json(serde_json::json!({
                        "data": post,
                        "threads_post_id": post_id,
                    })),
                ),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e.to_string(), "code": "CREATE_FAILED" })),
                ),
            }
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": e.to_string(), "code": "THREADS_API_ERROR" })),
        ),
    }
}

pub async fn delete_post(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let post = match state.store.get_post(&id).await {
        Ok(p) => p,
        Err(e) => return Json(serde_json::json!({ "error": e.to_string(), "code": "NOT_FOUND" })),
    };

    if let Some(threads_post_id) = &post.threads_post_id {
        if let Ok(account) = state.store.get_account(&post.account_id).await {
            if let Err(e) = state
                .threads_client
                .delete_post(&account, threads_post_id)
                .await
            {
                tracing::warn!(
                    "Failed to delete Threads post {threads_post_id} for post {id}: {e}"
                );
            }
        }
    }

    match state.store.delete_post(&id).await {
        Ok(()) => Json(serde_json::json!({ "data": null })),
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "DELETE_FAILED" })),
    }
}

pub async fn get_insights(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let post = match state.store.get_post(&id).await {
        Ok(p) => p,
        Err(e) => return Json(serde_json::json!({ "error": e.to_string(), "code": "NOT_FOUND" })),
    };

    let threads_post_id = match &post.threads_post_id {
        Some(id) => id,
        None => {
            return Json(
                serde_json::json!({ "error": "Post not yet published to Threads", "code": "NOT_PUBLISHED" }),
            );
        }
    };

    match state.store.get_account(&post.account_id).await {
        Ok(account) => {
            match state
                .threads_client
                .fetch_insights(&account, threads_post_id, None)
                .await
            {
                Ok(insights) => {
                    // Store snapshot
                    let snap_id = Uuid::now_v7().to_string();
                    let insights_model: titen_core::models::Insights = insights.into();
                    if let Err(e) = state
                        .store
                        .insert_analytics_snap(&snap_id, &id, &insights_model)
                        .await
                    {
                        tracing::warn!("Failed to store analytics snapshot for post {id}: {e}");
                    }
                    Json(serde_json::json!({ "data": insights_model }))
                }
                Err(e) => {
                    Json(serde_json::json!({ "error": e.to_string(), "code": "INSIGHTS_FAILED" }))
                }
            }
        }
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "ACCOUNT_NOT_FOUND" })),
    }
}
