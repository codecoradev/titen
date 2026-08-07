use axum::{Json, extract::Path, extract::State};
use uuid::Uuid;

use crate::server::AppState;
use titen_core::sentiment::build_engine;

pub async fn list_comments(
    State(state): State<AppState>,
    Path(post_id): Path<String>,
) -> Json<serde_json::Value> {
    match state.store.list_comments(&post_id).await {
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
    let comments = match state.store.list_comments(&post_id).await {
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
    let comments = match state.store.list_comments(&post_id).await {
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
