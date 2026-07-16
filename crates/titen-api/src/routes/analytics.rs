use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;

use crate::server::AppState;
use titen_core::models::*;

#[derive(Deserialize)]
pub struct AnalyticsQuery {
    pub account_id: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

pub async fn list_analytics(
    State(state): State<AppState>,
    Query(_q): Query<AnalyticsQuery>,
) -> Json<serde_json::Value> {
    // TODO: implement aggregated analytics
    Json(serde_json::json!({
        "data": {
            "total_posts": 0,
            "total_likes": 0,
            "total_replies": 0,
            "total_reposts": 0,
            "total_views": 0,
            "posts": []
        }
    }))
}

pub async fn post_trend(
    State(state): State<AppState>,
    Path(post_id): Path<String>,
) -> Json<serde_json::Value> {
    match state.store.list_analytics_snap(&post_id).await {
        Ok(snaps) => Json(serde_json::json!({ "data": snaps })),
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "FETCH_FAILED" })),
    }
}
