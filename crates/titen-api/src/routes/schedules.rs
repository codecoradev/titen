use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::server::AppState;
use titen_core::models::*;

#[derive(Deserialize)]
pub struct ScheduleListQuery {
    pub account_id: Option<String>,
    pub status: Option<String>,
}

pub async fn list_schedules(
    State(state): State<AppState>,
    Query(q): Query<ScheduleListQuery>,
) -> Json<serde_json::Value> {
    match state
        .store
        .list_schedules(q.account_id.as_deref(), q.status.as_deref())
        .await
    {
        Ok(schedules) => Json(serde_json::json!({ "data": schedules })),
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "LIST_FAILED" })),
    }
}

pub async fn list_upcoming(State(state): State<AppState>) -> Json<serde_json::Value> {
    match state.store.list_schedules(None, Some("pending")).await {
        Ok(schedules) => {
            let upcoming: Vec<_> = schedules.into_iter().take(10).collect();
            Json(serde_json::json!({ "data": upcoming }))
        }
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "LIST_FAILED" })),
    }
}

pub async fn create_schedule(
    State(state): State<AppState>,
    Json(input): Json<CreateSchedule>,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    let id = Uuid::now_v7().to_string();
    match state.store.create_schedule(&id, &input).await {
        Ok(schedule) => (
            axum::http::StatusCode::CREATED,
            Json(serde_json::json!({ "data": schedule })),
        ),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string(), "code": "CREATE_FAILED" })),
        ),
    }
}

pub async fn update_schedule(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<CreateSchedule>,
) -> Json<serde_json::Value> {
    // TODO: implement schedule update
    Json(serde_json::json!({ "message": "Schedule update not yet implemented", "id": id }))
}

pub async fn delete_schedule(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match state.store.delete_schedule(&id).await {
        Ok(()) => Json(serde_json::json!({ "data": null })),
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "DELETE_FAILED" })),
    }
}
