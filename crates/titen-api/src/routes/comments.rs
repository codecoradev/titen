use axum::{Json, extract::Path, extract::State};

use crate::server::AppState;

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
    State(_state): State<AppState>,
    Path(post_id): Path<String>,
) -> Json<serde_json::Value> {
    // TODO: implement actual Threads API comment fetching
    Json(serde_json::json!({ "message": "Comment fetch not yet implemented", "post_id": post_id }))
}

pub async fn get_sentiment(
    State(state): State<AppState>,
    Path(post_id): Path<String>,
) -> Json<serde_json::Value> {
    // TODO: implement sentiment analysis
    match state.store.list_comments(&post_id).await {
        Ok(comments) => {
            let total = comments.len() as i64;
            let analyzed: Vec<_> = comments
                .into_iter()
                .filter(|c| c.sentiment.is_some())
                .collect();
            Json(serde_json::json!({
                "data": {
                    "total": total,
                    "analyzed": analyzed.len(),
                    "positive": 0,
                    "negative": 0,
                    "neutral": 0,
                    "average_score": 0.0,
                    "comments": analyzed
                }
            }))
        }
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "FETCH_FAILED" })),
    }
}
