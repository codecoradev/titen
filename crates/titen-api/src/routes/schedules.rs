use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::server::AppState;
use titen_core::models::*;

#[utoipa::path(
    get,
    path = "/api/schedules",
    tag = "schedules",
    params(("filter" = Option<ScheduleFilter>, Query, description = "Schedule filter")),
    responses(
        (status = 200, description = "List of schedules", body = serde_json::Value),
    ),
    security(("api_key" = [])),
)]
pub async fn list_schedules(
    State(state): State<AppState>,
    Query(filter): Query<ScheduleFilter>,
) -> Json<serde_json::Value> {
    match state.store.list_schedules(&filter).await {
        Ok(schedules) => Json(serde_json::json!({ "data": schedules })),
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "LIST_FAILED" })),
    }
}

#[utoipa::path(
    get,
    path = "/api/schedules/{id}",
    tag = "schedules",
    params(("id" = String, Path, description = "Schedule ID")),
    responses(
        (status = 200, description = "Schedule details", body = serde_json::Value),
        (status = 404, description = "Not found", body = serde_json::Value),
    ),
    security(("api_key" = [])),
)]
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

#[utoipa::path(
    get,
    path = "/api/schedules/upcoming",
    tag = "schedules",
    responses(
        (status = 200, description = "Upcoming schedules", body = serde_json::Value),
    ),
    security(("api_key" = [])),
)]
pub async fn list_upcoming(State(state): State<AppState>) -> Json<serde_json::Value> {
    // #117 fix: list_upcoming should return only future schedules, not all pending.
    // Sort by scheduled_at ascending and filter to only upcoming items.
    match state
        .store
        .list_schedules(&ScheduleFilter {
            status: Some("pending".to_string()),
            ..Default::default()
        })
        .await
    {
        Ok(schedules) => {
            let now = chrono::Utc::now();
            let upcoming: Vec<_> = schedules
                .into_iter()
                .filter(|s| {
                    // Parse scheduled_at — only include items in the future
                    chrono::DateTime::parse_from_rfc3339(&s.scheduled_at)
                        .map(|dt| dt.with_timezone(&chrono::Utc) > now)
                        .unwrap_or(true) // include if we can't parse (defensive)
                })
                .take(10)
                .collect();
            Json(serde_json::json!({ "data": upcoming }))
        }
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "LIST_FAILED" })),
    }
}

#[utoipa::path(
    post,
    path = "/api/schedules",
    tag = "schedules",
    request_body = CreateSchedule,
    responses(
        (status = 201, description = "Schedule created", body = serde_json::Value),
        (status = 500, description = "Internal server error", body = serde_json::Value),
    ),
    security(("api_key" = [])),
)]
pub async fn create_schedule(
    State(state): State<AppState>,
    Json(input): Json<CreateSchedule>,
) -> (StatusCode, Json<serde_json::Value>) {
    // #186: Validate media_urls are absolute URLs. Relative paths (e.g.
    // "/2026/08/09/uuid.png") will cause Threads API publish failures.
    if let Some(ref urls) = input.media_urls {
        for url in urls {
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": format!("media_urls must be absolute URLs (http/https), got: {url}"),
                        "code": "INVALID_MEDIA_URL"
                    })),
                );
            }
        }
    }
    // #136: Validate caption length against Threads API limit (500 chars).
    // Check both caption and text_attachment since they are merged downstream.
    if let Some(ref c) = input.caption {
        if c.chars().count() > 500 {
            return caption_too_long(c);
        }
    }
    if let Some(ref t) = input.text_attachment {
        if t.chars().count() > 500 {
            return caption_too_long(t);
        }
    }

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

#[utoipa::path(
    patch,
    path = "/api/schedules/{id}",
    tag = "schedules",
    params(("id" = String, Path, description = "Schedule ID")),
    request_body = UpdateSchedule,
    responses(
        (status = 200, description = "Schedule patched", body = serde_json::Value),
        (status = 404, description = "Not found", body = serde_json::Value),
        (status = 409, description = "Conflict — invalid state", body = serde_json::Value),
    ),
    security(("api_key" = [])),
)]
pub async fn patch_schedule(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<UpdateSchedule>,
) -> (StatusCode, Json<serde_json::Value>) {
    // #186: Validate media_urls are absolute URLs (same as create).
    if let Some(ref urls) = input.media_urls {
        for url in urls {
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": format!("media_urls must be absolute URLs (http/https), got: {url}"),
                        "code": "INVALID_MEDIA_URL"
                    })),
                );
            }
        }
    }
    // #136: Validate caption length.
    if let Some(ref c) = input.caption {
        if c.chars().count() > 500 {
            return caption_too_long(c);
        }
    }
    match state
        .store
        .update_schedule_fields(
            &id,
            input.caption.as_deref(),
            input.media_type.as_deref(),
            input.media_urls,
            input.scheduled_at.as_deref(),
            input.location_id.as_deref(),
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

#[utoipa::path(
    put,
    path = "/api/schedules/{id}",
    tag = "schedules",
    params(("id" = String, Path, description = "Schedule ID")),
    request_body = CreateSchedule,
    responses(
        (status = 200, description = "Schedule updated", body = serde_json::Value),
        (status = 404, description = "Not found", body = serde_json::Value),
    ),
    security(("api_key" = [])),
)]
pub async fn update_schedule(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<CreateSchedule>,
) -> (StatusCode, Json<serde_json::Value>) {
    // #113 fix: Replaced delete+recreate with direct update to prevent data loss.
    // The old implementation deleted the schedule (losing created_at, result_json,
    // published_at, etc.) and recreated it. Now we delegate to update_schedule_fields
    // which preserves existing data via COALESCE.
    //
    // #114 fix: update_schedule_fields WHERE clause restricts to status='draft',
    // so pending/published schedules are correctly rejected.
    //
    // Merge text_attachment → caption: CreateSchedule has both fields, but the
    // store layer only knows about caption. Use text_attachment as fallback.
    let effective_caption = input.caption.or(input.text_attachment);

    // #186: Validate media_urls are absolute URLs (same as create).
    if let Some(ref urls) = input.media_urls {
        for url in urls {
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": format!("media_urls must be absolute URLs (http/https), got: {url}"),
                        "code": "INVALID_MEDIA_URL"
                    })),
                );
            }
        }
    }

    // #136: Validate caption length.
    if let Some(ref c) = effective_caption {
        if c.chars().count() > 500 {
            return caption_too_long(c);
        }
    }

    match state
        .store
        .update_schedule_fields(
            &id,
            effective_caption.as_deref(),
            input.media_type.as_deref(),
            input.media_urls.clone(),
            Some(&input.scheduled_at),
            input.location_id.as_deref(),
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

#[utoipa::path(
    delete,
    path = "/api/schedules/{id}",
    tag = "schedules",
    params(("id" = String, Path, description = "Schedule ID")),
    responses(
        (status = 200, description = "Schedule deleted", body = serde_json::Value),
        (status = 404, description = "Not found", body = serde_json::Value),
    ),
    security(("api_key" = [])),
)]
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

#[utoipa::path(
    post,
    path = "/api/schedules/{id}/approve",
    tag = "schedules",
    params(("id" = String, Path, description = "Schedule ID")),
    responses(
        (status = 200, description = "Schedule approved", body = serde_json::Value),
        (status = 404, description = "Not found", body = serde_json::Value),
        (status = 409, description = "Conflict — invalid state", body = serde_json::Value),
    ),
    security(("api_key" = [])),
)]
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

#[utoipa::path(
    post,
    path = "/api/schedules/{id}/reject",
    tag = "schedules",
    params(("id" = String, Path, description = "Schedule ID")),
    responses(
        (status = 200, description = "Schedule rejected", body = serde_json::Value),
        (status = 404, description = "Not found", body = serde_json::Value),
        (status = 409, description = "Conflict — invalid state", body = serde_json::Value),
    ),
    security(("api_key" = [])),
)]
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

/// #136: Return 400 for captions exceeding Threads API 500-character limit.
/// Uses char count (not byte count) to match Threads API semantics.
fn caption_too_long(caption: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({
            "error": format!(
                "Caption exceeds Threads API limit of 500 characters (got {})",
                caption.chars().count()
            ),
            "code": "CAPTION_TOO_LONG"
        })),
    )
}
