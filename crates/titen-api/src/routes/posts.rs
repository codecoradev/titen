use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::server::AppState;
use titen_core::models::*;

/// Validate that a URL is HTTPS and well-formed.
/// Meta Threads API requires HTTPS for all media URLs.
fn validate_media_url(url: &str) -> Result<(), String> {
    let parsed = url::Url::parse(url).map_err(|e| format!("Invalid URL: {e}"))?;
    if parsed.scheme() != "https" {
        return Err(format!(
            "Media URL must use HTTPS, got '{}'",
            parsed.scheme()
        ));
    }
    let host = parsed.host_str().ok_or("URL missing host")?;
    if host.is_empty() {
        return Err("URL host is empty".to_string());
    }

    // Block internal/private addresses to prevent SSRF.
    // Handles hostname, IPv4 (including decimal/octal/hex encoding), and IPv6.
    let is_internal = is_internal_host(host);

    if is_internal {
        return Err("Internal addresses are not allowed for media URLs".to_string());
    }
    Ok(())
}

/// Check if a host string resolves to or represents an internal/private address.
/// Handles: hostnames, IPv4 literals (decimal/octal/hex), IPv6 literals, and bracketed IPv6.
fn is_internal_host(host: &str) -> bool {
    // Strip brackets from IPv6 format [::1]
    let host = host.trim_start_matches('[').trim_end_matches(']');

    // Common internal hostnames
    if host == "localhost" || host.is_empty() {
        return true;
    }

    // Try parsing as IP address — handles decimal (2130706433), octal (0177.0.0.1),
    // hex (0x7f000001), and standard IPv4/IPv6 formats.
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        return is_internal_ip(&ip);
    }

    // Check raw string for private IPv4 ranges (for non-standard formats that std::net doesn't parse)
    host == "0.0.0.0"
        || host.starts_with("10.")
        || host.starts_with("192.168.")
        || (host.starts_with("172.") && {
            if let Some(second) = host.split('.').nth(1) {
                second.parse::<u8>().is_ok_and(|n| (16..=31).contains(&n))
            } else {
                false
            }
        })
        || host.starts_with("169.254.")
        || host.starts_with("127.")
}

/// Check if an IP address is internal/private/loopback.
fn is_internal_ip(ip: &std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
        }
        std::net::IpAddr::V6(v6) => {
            v6.is_loopback() || v6.is_unspecified() || {
                // Check for IPv4-mapped IPv6 (::ffff:a.b.c.d)
                v6.to_ipv4()
                    .is_some_and(|v4| v4.is_loopback() || v4.is_private() || v4.is_link_local())
            }
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/posts",
    tag = "posts",
    params(("filter" = Option<PostFilter>, Query, description = "Post filter")),
    responses(
        (status = 200, description = "List of posts", body = serde_json::Value),
    ),
    security(("api_key" = [])),
)]
pub async fn list_posts(
    State(state): State<AppState>,
    Query(filter): Query<PostFilter>,
) -> Json<serde_json::Value> {
    match state.store.list_posts(&filter).await {
        Ok(posts) => Json(serde_json::json!({ "data": posts })),
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "LIST_FAILED" })),
    }
}

#[utoipa::path(
    get,
    path = "/api/posts/{id}",
    tag = "posts",
    params(("id" = String, Path, description = "Post ID")),
    responses(
        (status = 200, description = "Post details", body = serde_json::Value),
        (status = 404, description = "Not found", body = serde_json::Value),
    ),
    security(("api_key" = [])),
)]
pub async fn get_post(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match state.store.get_post(&id).await {
        Ok(post) => Json(serde_json::json!({ "data": post })),
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "NOT_FOUND" })),
    }
}

#[utoipa::path(
    post,
    path = "/api/posts",
    tag = "posts",
    request_body = CreatePost,
    responses(
        (status = 201, description = "Post created", body = serde_json::Value),
        (status = 400, description = "Bad request", body = serde_json::Value),
        (status = 404, description = "Account not found", body = serde_json::Value),
        (status = 429, description = "Rate limited", body = serde_json::Value),
        (status = 502, description = "Threads API error", body = serde_json::Value),
    ),
    security(("api_key" = [])),
)]
pub async fn create_post(
    State(state): State<AppState>,
    Json(input): Json<CreatePost>,
) -> (StatusCode, Json<serde_json::Value>) {
    // #136: Validate caption length against Threads API limit (500 chars).
    if let Some(ref c) = input.caption {
        if c.chars().count() > 500 {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!(
                        "Caption exceeds Threads API limit of 500 characters (got {})",
                        c.chars().count()
                    ),
                    "code": "CAPTION_TOO_LONG"
                })),
            );
        }
    }

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

    // P3.8: Validate all media URLs (HTTPS-only, no internal addresses).
    // Prevents SSRF if server-side fetch is ever added and aligns with Meta API requirements.
    if let Some(ref url) = effective_input.image_url {
        if let Err(e) = validate_media_url(url) {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e, "code": "INVALID_MEDIA_URL" })),
            );
        }
    }
    if let Some(ref url) = effective_input.video_url {
        if let Err(e) = validate_media_url(url) {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e, "code": "INVALID_MEDIA_URL" })),
            );
        }
    }
    if let Some(ref urls) = effective_input.image_urls {
        for url in urls {
            if let Err(e) = validate_media_url(url) {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "error": e, "code": "INVALID_MEDIA_URL" })),
                );
            }
        }
    }

    // Publish via Threads API
    let caption = effective_input.caption.as_deref().unwrap_or("");
    let threads_post_id = match effective_input.media_type.as_deref().unwrap_or("TEXT") {
        "TEXT" => {
            state
                .threads_client
                .publish_text(&account, caption, None)
                .await
        }
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
                        None,
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
                    .publish_video(&account, Some(caption), url, None)
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
            // Fetch permalink best-effort (non-fatal if it fails).
            let permalink = state
                .threads_client
                .get_permalink(&account, &post_id)
                .await
                .ok()
                .flatten();
            match state
                .store
                .create_post_with_threads_id(&db_id, &effective_input, &post_id, permalink.as_deref())
                .await
            {
                Ok(post) => (
                    StatusCode::CREATED,
                    Json(serde_json::json!({
                        "data": post,
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

#[utoipa::path(
    delete,
    path = "/api/posts/{id}",
    tag = "posts",
    params(("id" = String, Path, description = "Post ID")),
    responses(
        (status = 200, description = "Post deleted", body = serde_json::Value),
        (status = 404, description = "Not found", body = serde_json::Value),
    ),
    security(("api_key" = [])),
)]
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

#[utoipa::path(
    get,
    path = "/api/posts/{id}/insights",
    tag = "posts",
    params(("id" = String, Path, description = "Post ID")),
    responses(
        (status = 200, description = "Post insights", body = serde_json::Value),
        (status = 404, description = "Not found", body = serde_json::Value),
    ),
    security(("api_key" = [])),
)]
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
