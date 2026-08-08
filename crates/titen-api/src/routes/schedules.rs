use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::server::AppState;
use titen_core::models::*;

pub async fn list_schedules(
    State(state): State<AppState>,
    Query(filter): Query<ScheduleFilter>,
) -> Json<serde_json::Value> {
    match state.store.list_schedules(&filter).await {
        Ok(schedules) => Json(serde_json::json!({ "data": schedules })),
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "LIST_FAILED" })),
    }
}

pub async fn get_schedule_by_id(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    match state.store.get_schedule(&id).await {
        Ok(schedule) => (
            StatusCode::OK,
            Json(serde_json::json!({ "data": schedule })),
        ),
        Err(e) => {
            let code = if matches!(e, titen_core::TitenError::ScheduleNotFound(_)) {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                code,
                Json(serde_json::json!({ "error": e.to_string(), "code": "NOT_FOUND" })),
            )
        }
    }
}

pub async fn list_upcoming(State(state): State<AppState>) -> Json<serde_json::Value> {
    match state
        .store
        .list_schedules(&ScheduleFilter {
            status: Some("pending".to_string()),
            ..Default::default()
        })
        .await
    {
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
) -> (StatusCode, Json<serde_json::Value>) {
    let id = Uuid::now_v7().to_string();
    match state.store.create_schedule(&id, &input).await {
        Ok(schedule) => (
            StatusCode::CREATED,
            Json(serde_json::json!({ "data": schedule })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string(), "code": "CREATE_FAILED" })),
        ),
    }
}

/// PATCH /api/schedules/{id} — partial update of editable fields.
/// Only works on schedules in 'draft' or 'pending' state.
pub async fn patch_schedule(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<UpdateSchedule>,
) -> (StatusCode, Json<serde_json::Value>) {
    match state
        .store
        .update_schedule_fields(
            &id,
            input.caption.as_deref(),
            input.media_type.as_deref(),
            input.media_urls,
            input.scheduled_at.as_deref(),
        )
        .await
    {
        Ok(schedule) => (
            StatusCode::OK,
            Json(serde_json::json!({ "data": schedule })),
        ),
        Err(e) => {
            let code = if matches!(e, titen_core::TitenError::ScheduleNotFound(_)) {
                StatusCode::NOT_FOUND
            } else if matches!(e, titen_core::TitenError::InvalidRequest(_)) {
                StatusCode::CONFLICT
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                code,
                Json(serde_json::json!({ "error": e.to_string(), "code": "UPDATE_FAILED" })),
            )
        }
    }
}

pub async fn update_schedule(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<CreateSchedule>,
) -> Json<serde_json::Value> {
    match state.store.delete_schedule(&id).await {
        Ok(()) => match state.store.create_schedule(&id, &input).await {
            Ok(schedule) => Json(serde_json::json!({ "data": schedule })),
            Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "UPDATE_FAILED" })),
        },
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "NOT_FOUND" })),
    }
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

// ─── HITL: Approve / Reject ──────────────────────────────

/// POST /api/schedules/{id}/approve
/// Transitions a schedule from 'draft' → 'pending' (ready for auto-publish).
pub async fn approve_schedule(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    // TODO: extract approver identity from auth context when per-user auth is added
    match state.store.approve_schedule(&id, Some("api")).await {
        Ok(schedule) => (
            StatusCode::OK,
            Json(serde_json::json!({ "data": schedule })),
        ),
        Err(e) => {
            let code = if matches!(e, titen_core::TitenError::ScheduleNotFound(_)) {
                StatusCode::NOT_FOUND
            } else if matches!(e, titen_core::TitenError::InvalidRequest(_)) {
                StatusCode::CONFLICT
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                code,
                Json(serde_json::json!({ "error": e.to_string(), "code": "APPROVE_FAILED" })),
            )
        }
    }
}

/// POST /api/schedules/{id}/reject
/// Transitions a schedule from 'draft' → 'rejected'.
#[derive(Deserialize)]
pub struct RejectBody {
    pub reason: Option<String>,
}

pub async fn reject_schedule(
    State(state): State<AppState>,
    Path(id): Path<String>,
    body: Option<Json<RejectBody>>,
) -> (StatusCode, Json<serde_json::Value>) {
    let reason = body.and_then(|b| b.reason.clone());
    match state.store.reject_schedule(&id, reason.as_deref()).await {
        Ok(schedule) => (
            StatusCode::OK,
            Json(serde_json::json!({ "data": schedule })),
        ),
        Err(e) => {
            let code = if matches!(e, titen_core::TitenError::ScheduleNotFound(_)) {
                StatusCode::NOT_FOUND
            } else if matches!(e, titen_core::TitenError::InvalidRequest(_)) {
                StatusCode::CONFLICT
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                code,
                Json(serde_json::json!({ "error": e.to_string(), "code": "REJECT_FAILED" })),
            )
        }
    }
}
