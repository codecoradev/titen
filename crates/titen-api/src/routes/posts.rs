use axum::{
    Json,
    extract::{Path, State, Query},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::server::AppState;
use titen_core::models::*;

#[derive(Deserialize)]
pub struct PostListQuery {
    pub account_id: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_posts(State(state): State<AppState>, Query(q): Query<PostListQuery>) -> Json<serde_json::Value> {
    let limit = q.limit.unwrap_or(50);
    let offset = q.offset.unwrap_or(0);
    match state.store.list_posts(q.account_id.as_deref(), q.status.as_deref(), limit, offset).await {
        Ok(posts) => Json(serde_json::json!({ "data": posts })),
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "LIST_FAILED" })),
    }
}

pub async fn get_post(State(state): State<AppState>, Path(id): Path<String>) -> Json<serde_json::Value> {
    match state.store.get_post(&id).await {
        Ok(post) => Json(serde_json::json!({ "data": post })),
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "NOT_FOUND" })),
    }
}

pub async fn create_post(State(state): State<AppState>, Json(input): Json<CreatePost>) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    let id = Uuid::now_v7().to_string();
    match state.store.create_post(&id, &input).await {
        Ok(post) => (axum::http::StatusCode::CREATED, Json(serde_json::json!({ "data": post }))),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string(), "code": "CREATE_FAILED" })),
        ),
    }
}

pub async fn delete_post(State(state): State<AppState>, Path(id): Path<String>) -> Json<serde_json::Value> {
    // TODO: implement actual Threads delete + remove from DB
    Json(serde_json::json!({ "message": "Post delete not yet implemented", "post_id": id }))
}

pub async fn get_insights(State(state): State<AppState>, Path(id): Path<String>) -> Json<serde_json::Value> {
    // TODO: implement Threads insights fetch
    match state.store.get_post(&id).await {
        Ok(post) => Json(serde_json::json!({ "data": post })),
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "NOT_FOUND" })),
    }
}
