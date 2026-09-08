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
    let limit = filter.limit.unwrap_or(50).clamp(1, 1000);
    let offset = filter.offset.unwrap_or(0).max(0);
    let total = state.store.count_schedules_filtered(&filter).await;
    match state.store.list_schedules(&filter).await {
        Ok(schedules) => Json(serde_json::json!({
            "data": schedules,
            "total": total.unwrap_or(schedules.len() as i64),
            "limit": limit,
            "offset": offset,
        })),
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
    // #136: Validate caption length (499-char guard, see caption_too_long).
    // Check both caption and text_attachment since they are merged downstream.
    if let Some(ref c) = input.caption {
        if c.chars().count() > 499 {
            return caption_too_long(c);
        }
    }
    if let Some(ref t) = input.text_attachment {
        if t.chars().count() > 499 {
            return caption_too_long(t);
        }
    }
    // scheduled_at must parse; otherwise the schedule would never become due.
    if let Err(resp) = ensure_valid_scheduled_at(&input.scheduled_at) {
        return resp;
    }
    // #232: reply schedules are TEXT-only and need a non-empty target.
    if let Some(ref r) = input.reply_to_id {
        if r.trim().is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "reply_to_id must be a non-empty Threads post ID",
                    "code": "INVALID_REPLY_TO_ID"
                })),
            );
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
        if c.chars().count() > 499 {
            return caption_too_long(c);
        }
    }
    // #232: reply target must be non-empty when provided. Media type is free:
    // Threads accepts reply_to_id on any container (Phase 1 removed TEXT-only).
    if let Some(r) = input.reply_to_id.as_deref() {
        if r.trim().is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "reply_to_id must be a non-empty Threads post ID",
                    "code": "INVALID_REPLY_TO_ID"
                })),
            );
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
            input.reply_to_id.as_deref(),
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
        if c.chars().count() > 499 {
            return caption_too_long(c);
        }
    }
    // scheduled_at must parse; otherwise the schedule would never become due.
    if let Err(resp) = ensure_valid_scheduled_at(&input.scheduled_at) {
        return resp;
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
            input.reply_to_id.as_deref(),
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
        Ok(schedule) => {
            // Thread-bundle HITL: approving the ROOT approves the whole
            // bundle — members 1..n (draft) become pending-waiting so the
            // chain runs after the root publishes. Approving a non-root
            // item only affects that item.
            if let Some(ref bundle_id) = schedule.bundle_id {
                if schedule.bundle_seq == Some(0) {
                    match state
                        .store
                        .set_bundle_status(bundle_id, "draft", "bundle_waiting")
                        .await
                    {
                        Ok(n) if n > 0 => tracing::info!(
                            "Bundle {bundle_id} approved: {n} waiting item(s) queued"
                        ),
                        Ok(_) => {}
                        Err(e) => tracing::error!("Bundle {bundle_id} approve cascade failed: {e}"),
                    }
                }
            }
            (
                StatusCode::OK,
                Json(serde_json::json!({ "data": schedule })),
            )
        }
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
        Ok(schedule) => {
            // Thread-bundle: rejecting one member strands the rest of the
            // chain. The cascade runs only after reject succeeds (reject
            // itself fails for non-rejectable states like published), so an
            // invalid request cannot destroy scheduled content.
            if let Some(ref bundle_id) = schedule.bundle_id {
                match state
                    .store
                    .fail_remaining_bundle(
                        bundle_id,
                        &format!(
                            "bundle chain rejected at seq {}: {}",
                            schedule.bundle_seq.unwrap_or(0),
                            reason.as_deref().unwrap_or("no reason")
                        ),
                    )
                    .await
                {
                    Ok(n) if n > 0 => {
                        tracing::info!("Bundle {bundle_id} rejected: failed {n} remaining item(s)");
                    }
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!("Bundle {bundle_id} cascade fail failed: {e}")
                    }
                }
            }
            (
                StatusCode::OK,
                Json(serde_json::json!({ "data": schedule })),
            )
        }
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

/// Validate scheduled_at parses as an ISO 8601 / RFC 3339 timestamp. The
/// scheduler compares the stored string against datetime('now'); an
/// unparseable value never becomes due and sits invisible in the queue.
fn ensure_valid_scheduled_at(value: &str) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if titen_core::time::parse_utc(value).is_some() {
        Ok(())
    } else {
        Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!(
                    "scheduled_at must be an ISO 8601 / RFC 3339 timestamp with offset, got: {value}"
                ),
                "code": "INVALID_SCHEDULED_AT"
            })),
        ))
    }
}

/// #136: Return 400 for captions exceeding Threads API 500-character limit.
/// Uses char count (not byte count) to match Threads API semantics.
fn caption_too_long(caption: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({
            "error": format!(
                "Caption exceeds the 499-character limit (got {}; Threads accepts at most 500)",
                caption.chars().count()
            ),
            "code": "CAPTION_TOO_LONG"
        })),
    )
}

// ─── #242: agent/CI ingest endpoint ─────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/schedules/ingest",
    tag = "schedules",
    request_body = IngestSchedule,
    responses(
        (status = 201, description = "Draft schedule ingested", body = serde_json::Value),
        (status = 400, description = "Validation error", body = serde_json::Value),
        (status = 500, description = "Internal server error", body = serde_json::Value),
    ),
    security(("api_key" = [])),
)]
/// Ingest a draft schedule from an external agent or CI pipeline (#242).
///
/// The schedule is ALWAYS created as `draft` — the human-approval gate is
/// non-negotiable for machine-originated content. `source` badges the item
/// with its origin so the calendar/UI can show where it came from.
pub async fn ingest_schedule(
    State(state): State<AppState>,
    Json(input): Json<IngestSchedule>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Auth: this route is registered inside `protected_routes` in server.rs and
    // sits behind the shared `api_key_auth` route_layer — same as every other
    // schedule endpoint. It has no per-handler auth extractor by design; the
    // integration tests use the middleware-free test router on purpose.
    use titen_core::models::CreateSchedule;

    // HITL gate: machine-originated drafts never auto-approve.
    let create = CreateSchedule {
        account_id: input.account_id,
        media_type: input.media_type,
        caption: input.caption,
        text_attachment: None,
        media_urls: input.media_urls,
        scheduled_at: input.scheduled_at,
        location_id: input.location_id,
        auto_approve: false,
        reply_to_id: input.reply_to_id,
    };

    // Reuse the same validation surface as the manual create route by
    // validating inline (media URLs, caption length, reply constraints).
    if let Some(ref urls) = create.media_urls {
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
    if let Some(ref c) = create.caption {
        if c.chars().count() > 499 {
            return caption_too_long(c);
        }
    }
    // scheduled_at must parse; otherwise the draft would never become due.
    if let Err(resp) = ensure_valid_scheduled_at(&create.scheduled_at) {
        return resp;
    }
    if let Some(ref r) = create.reply_to_id {
        if r.trim().is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "reply_to_id must be a non-empty Threads post ID",
                    "code": "INVALID_REPLY_TO_ID"
                })),
            );
        }
    }
    // Source identifier: required-ish (agents should tag origin), capped at a
    // sane length so it can't smuggle unbounded data.
    let source = input
        .source
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if let Some(src) = source {
        if src.chars().count() > 64 {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "source must be at most 64 characters",
                    "code": "INVALID_SOURCE"
                })),
            );
        }
    }

    let id = Uuid::now_v7().to_string();
    match state
        .store
        .create_schedule_with_source(&id, &create, source)
        .await
    {
        Ok(schedule) => (
            StatusCode::CREATED,
            Json(serde_json::json!({
                "data": schedule,
                "note": "Created as draft — human approval required before publishing."
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string(), "code": "INGEST_FAILED" })),
        ),
    }
}
