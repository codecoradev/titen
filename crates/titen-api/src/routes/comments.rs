use axum::{
    Json,
    extract::State,
    extract::{Path, Query},
    http::StatusCode,
};
use uuid::Uuid;

use crate::server::AppState;
use titen_core::models::{CommentFilter, UpdateCommentReply};
use titen_core::sentiment::build_engine;

pub async fn list_comments(
    State(state): State<AppState>,
    Path(post_id): Path<String>,
    Query(filter): Query<CommentFilter>,
) -> Json<serde_json::Value> {
    match state.store.list_comments(&post_id, &filter).await {
        Ok(comments) => Json(serde_json::json!({ "data": comments })),
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "LIST_FAILED" })),
    }
}

pub async fn fetch_comments(
    State(state): State<AppState>,
    Path(post_id): Path<String>,
) -> Json<serde_json::Value> {
    // Get the post to find account and threads_post_id
    let post = match state.store.get_post(&post_id).await {
        Ok(p) => p,
        Err(e) => return Json(serde_json::json!({ "error": e.to_string(), "code": "NOT_FOUND" })),
    };

    let threads_post_id = match &post.threads_post_id {
        Some(id) => id.clone(),
        None => {
            return Json(serde_json::json!({
                "error": "Post not yet published to Threads",
                "code": "NOT_PUBLISHED"
            }));
        }
    };

    let account = match state.store.get_account(&post.account_id).await {
        Ok(a) => a,
        Err(e) => {
            return Json(
                serde_json::json!({ "error": e.to_string(), "code": "ACCOUNT_NOT_FOUND" }),
            );
        }
    };

    // Fetch from Threads API
    let comment_data = match state
        .threads_client
        .fetch_comments(&account, &threads_post_id)
        .await
    {
        Ok(data) => data,
        Err(e) => {
            return Json(serde_json::json!({ "error": e.to_string(), "code": "FETCH_FAILED" }));
        }
    };

    // Store in DB
    let mut stored = Vec::new();
    for cd in &comment_data {
        let id = Uuid::now_v7().to_string();
        match state
            .store
            .insert_comment(
                &id,
                &post_id,
                cd.author_username.as_deref(),
                cd.author_user_id.as_deref(),
                &cd.text,
            )
            .await
        {
            Ok(c) => stored.push(c),
            Err(e) => {
                // Skip duplicates or errors, continue with others
                tracing::warn!("Failed to store comment {id}: {e}");
            }
        }
    }

    Json(serde_json::json!({
        "data": stored,
        "fetched": comment_data.len(),
        "stored": stored.len(),
    }))
}

pub async fn get_sentiment(
    State(state): State<AppState>,
    Path(post_id): Path<String>,
) -> Json<serde_json::Value> {
    let comments = match state
        .store
        .list_comments(&post_id, &CommentFilter::default())
        .await
    {
        Ok(c) => c,
        Err(e) => {
            return Json(serde_json::json!({ "error": e.to_string(), "code": "FETCH_FAILED" }));
        }
    };

    let total = comments.len() as i64;

    // Find comments without sentiment analysis
    let unanalyzed: Vec<String> = comments
        .iter()
        .filter(|c| c.sentiment.is_none())
        .map(|c| c.text.clone())
        .collect();

    // Run sentiment engine
    let engine_type =
        std::env::var("TITEN_SENTIMENT_ENGINE").unwrap_or_else(|_| "keyword".to_string());
    let engine = build_engine(&engine_type);

    if !unanalyzed.is_empty() {
        let refs: Vec<&str> = unanalyzed.iter().map(|s| s.as_str()).collect();
        match engine.analyze_batch(&refs).await {
            Ok(results) => {
                for (comment, result) in comments.iter().zip(results.iter()) {
                    if comment.sentiment.is_none() {
                        if let Err(e) = state
                            .store
                            .update_comment_sentiment(&comment.id, &result.label, result.score)
                            .await
                        {
                            tracing::debug!(
                                "Failed to update sentiment for comment {}: {e}",
                                comment.id
                            );
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Sentiment analysis failed: {e}");
            }
        }
    }

    // Re-fetch with updated sentiments
    let comments = match state
        .store
        .list_comments(&post_id, &CommentFilter::default())
        .await
    {
        Ok(c) => c,
        Err(_) => comments.clone(),
    };

    let analyzed: Vec<_> = comments.iter().filter(|c| c.sentiment.is_some()).collect();

    let positive = analyzed
        .iter()
        .filter(|c| c.sentiment.as_deref() == Some("positive"))
        .count() as i64;
    let negative = analyzed
        .iter()
        .filter(|c| c.sentiment.as_deref() == Some("negative"))
        .count() as i64;
    let neutral = analyzed
        .iter()
        .filter(|c| c.sentiment.as_deref() == Some("neutral"))
        .count() as i64;
    let avg_score: f64 = analyzed
        .iter()
        .map(|c| c.sentiment_score.unwrap_or(0.0))
        .sum::<f64>()
        / analyzed.len().max(1) as f64;

    Json(serde_json::json!({
        "data": {
            "total": total,
            "analyzed": analyzed.len(),
            "positive": positive,
            "negative": negative,
            "neutral": neutral,
            "average_score": avg_score,
            "comments": analyzed,
        }
    }))
}

/// PATCH /api/comments/{id} — update reply status (manual workflow).
pub async fn update_reply_status(
    State(state): State<AppState>,
    Path(comment_id): Path<String>,
    Json(body): Json<UpdateCommentReply>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Validate reply_status if provided
    let valid_statuses = ["new", "needs_reply", "replied", "skipped"];
    let reply_status = match body.reply_status.as_deref() {
        Some(s) if valid_statuses.contains(&s) => s.to_string(),
        Some(s) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("Invalid reply_status: {s}. Must be one of: new, needs_reply, replied, skipped"),
                    "code": "INVALID_STATUS"
                })),
            ));
        }
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "reply_status is required",
                    "code": "MISSING_FIELD"
                })),
            ));
        }
    };

    match state
        .store
        .update_comment_reply(&comment_id, &reply_status, body.reply_text.as_deref())
        .await
    {
        Ok(comment) => Ok(Json(serde_json::json!({ "data": comment }))),
        Err(e) => {
            let msg = e.to_string();
            let is_not_found = msg.contains("not found");
            let status = if is_not_found {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            let code = if is_not_found {
                "NOT_FOUND"
            } else {
                "UPDATE_FAILED"
            };
            Err((
                status,
                Json(serde_json::json!({ "error": msg, "code": code })),
            ))
        }
    }
}

/// POST /api/comments/{id}/reply — publish a reply to Threads and mark as replied.
pub async fn reply_to_comment(
    State(state): State<AppState>,
    Path(comment_id): Path<String>,
    Json(body): Json<UpdateCommentReply>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let reply_text = match body.reply_text.as_deref() {
        Some(t) if !t.trim().is_empty() => t.to_string(),
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "reply_text is required and must not be empty",
                    "code": "MISSING_FIELD"
                })),
            ));
        }
    };

    // Fetch the comment to get threads_comment_id
    let comment = state.store.get_comment(&comment_id).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": e.to_string(), "code": "NOT_FOUND" })),
        )
    })?;

    let threads_comment_id = comment.threads_comment_id.clone().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Comment has no threads_comment_id — cannot reply",
                "code": "NO_THREADS_ID"
            })),
        )
    })?;

    // Fetch the post → account for Threads API access
    let post = state.store.get_post(&comment.post_id).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": e.to_string(), "code": "POST_NOT_FOUND" })),
        )
    })?;

    let account = state
        .store
        .get_account(&post.account_id)
        .await
        .map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e.to_string(), "code": "ACCOUNT_NOT_FOUND" })),
            )
        })?;

    // Publish reply to Threads
    match state
        .threads_client
        .create_reply(&account, &threads_comment_id, &reply_text)
        .await
    {
        Ok(threads_reply_id) => {
            // Update DB: mark replied + store reply text
            match state
                .store
                .update_comment_reply(&comment_id, "replied", Some(&reply_text))
                .await
            {
                Ok(updated) => Ok(Json(serde_json::json!({
                    "data": updated,
                    "threads_reply_id": threads_reply_id,
                }))),
                Err(e) => Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": format!("Reply published to Threads but DB update failed: {e}"),
                        "code": "DB_UPDATE_FAILED",
                        "threads_reply_id": threads_reply_id,
                    })),
                )),
            }
        }
        Err(e) => Err((
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({
                "error": format!("Threads API reply failed: {e}"),
                "code": "THREADS_REPLY_FAILED"
            })),
        )),
    }
}
