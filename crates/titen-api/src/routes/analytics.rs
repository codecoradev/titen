use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;

use crate::server::AppState;
use titen_core::models::PostFilter;

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct AnalyticsQuery {
    pub account_id: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/analytics/posts",
    tag = "analytics",
    responses(
        (status = 200, description = "Aggregated post analytics", body = serde_json::Value),
    ),
    security(("api_key" = [])),
)]
pub async fn list_analytics(
    State(state): State<AppState>,
    Query(q): Query<AnalyticsQuery>,
) -> Json<serde_json::Value> {
    let posts = match state
        .store
        .list_posts(&PostFilter {
            account_id: q.account_id.clone(),
            status: Some("published".to_string()),
            from: q.from.clone(),
            to: q.to.clone(),
            limit: Some(1000),
            ..Default::default()
        })
        .await
    {
        Ok(p) => p,
        Err(e) => {
            return Json(serde_json::json!({ "error": e.to_string(), "code": "FETCH_FAILED" }));
        }
    };

    let mut total_likes = 0i64;
    let mut total_replies = 0i64;
    let mut total_reposts = 0i64;
    let mut total_views = 0i64;
    let mut total_quotes = 0i64;
    let mut post_data = Vec::new();

    for post in &posts {
        // Get latest analytics snap for this post
        let snaps = state
            .store
            .list_analytics_snap(&post.id)
            .await
            .unwrap_or_default();
        let latest = snaps.last();

        let likes = latest.map(|s| s.likes).unwrap_or(0);
        let replies = latest.map(|s| s.replies).unwrap_or(0);
        let reposts = latest.map(|s| s.reposts).unwrap_or(0);
        let views = latest.map(|s| s.views).unwrap_or(0);
        let quotes = latest.map(|s| s.quotes).unwrap_or(0);

        total_likes += likes;
        total_replies += replies;
        total_reposts += reposts;
        total_views += views;
        total_quotes += quotes;

        post_data.push(serde_json::json!({
            "post_id": post.id,
            "caption": post.caption,
            "threads_post_id": post.threads_post_id,
            "likes": likes,
            "replies": replies,
            "reposts": reposts,
            "views": views,
            "quotes": quotes,
        }));
    }

    Json(serde_json::json!({
        "data": {
            "total_posts": posts.len(),
            "total_likes": total_likes,
            "total_replies": total_replies,
            "total_reposts": total_reposts,
            "total_views": total_views,
            "total_quotes": total_quotes,
            "period": {
                "from": q.from.as_deref().unwrap_or("all"),
                "to": q.to.as_deref().unwrap_or("now"),
            },
            "posts": post_data,
        }
    }))
}

#[utoipa::path(
    get,
    path = "/api/analytics/posts/{id}/trend",
    tag = "analytics",
    params(("id" = String, Path, description = "Post ID")),
    responses(
        (status = 200, description = "Post analytics trend over time", body = serde_json::Value),
    ),
    security(("api_key" = [])),
)]
pub async fn post_trend(
    State(state): State<AppState>,
    Path(post_id): Path<String>,
) -> Json<serde_json::Value> {
    match state.store.list_analytics_snap(&post_id).await {
        Ok(snaps) => Json(serde_json::json!({ "data": snaps })),
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "FETCH_FAILED" })),
    }
}
