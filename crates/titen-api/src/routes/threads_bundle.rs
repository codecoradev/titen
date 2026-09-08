//! Thread bundles (#thread-bundle Phase 2): one root post plus chained
//! replies, publishable instantly or as a single scheduled slot.

use axum::{Json, extract::State, http::StatusCode};
use uuid::Uuid;

use titen_core::models::CreateSchedule;

/// Validate a single bundle item's media configuration (cheap, pre-insert).
fn validate_item(
    item: &titen_core::models::BundleItem,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let media_type = item.media_type.as_deref().unwrap_or("TEXT");
    let urls: Vec<String> = item.media_urls.clone().unwrap_or_default();
    match media_type {
        "TEXT" => {
            if item.caption.as_deref().unwrap_or("").trim().is_empty() {
                return Err(bad("TEXT items require a caption", "INVALID_ITEM"));
            }
        }
        "IMAGE" => {
            if urls.len() != 1 {
                return Err(bad(
                    "IMAGE items require exactly 1 media_urls entry",
                    "INVALID_ITEM",
                ));
            }
        }
        "VIDEO" => {
            if urls.len() != 1 {
                return Err(bad(
                    "VIDEO items require exactly 1 media_urls entry",
                    "INVALID_ITEM",
                ));
            }
        }
        "CAROUSEL" => {
            if !(2..=20).contains(&urls.len()) {
                return Err(bad(
                    "CAROUSEL items require 2-20 media_urls entries",
                    "INVALID_ITEM",
                ));
            }
        }
        other => {
            return Err(bad(
                &format!("Unsupported media_type: {other}"),
                "INVALID_ITEM",
            ));
        }
    }
    // Reply targets must reference an earlier item or an external post.
    if let Some(serde_json::Value::Number(n)) = &item.reply_to {
        let idx = n.as_i64().unwrap_or(i64::MIN);
        if idx < 0 {
            return Err(bad("reply_to index must be >= 0", "INVALID_REPLY_TO"));
        }
    }
    Ok(())
}

fn bad(message: &str, code: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({ "error": message, "code": code })),
    )
}

/// POST /api/threads — create a thread bundle.
///
/// Without `scheduled_at`: every item is created `pending` with the same
/// "now"-ish timestamp; the scheduler publishes the root, then the chain
/// hooks promote each next item as the previous one publishes.
/// With `scheduled_at`: all items share that slot; root is `pending`
/// (or `draft` unless auto_approve), the rest wait as `bundle_waiting`.
pub async fn create_thread_bundle(
    State(state): State<crate::server::AppState>,
    Json(input): Json<titen_core::models::CreateThreadBundle>,
) -> (StatusCode, Json<serde_json::Value>) {
    use titen_core::time::parse_utc;

    let n_items = input.posts.len();
    if n_items == 0 {
        return bad("posts must contain at least 1 item", "EMPTY_BUNDLE");
    }
    if n_items > 20 {
        return bad("posts is limited to 20 items", "TOO_MANY_ITEMS");
    }
    // Index-based reply targets must point at earlier items.
    for (i, item) in input.posts.iter().enumerate() {
        if let Some(serde_json::Value::Number(num)) = &item.reply_to {
            let idx = num.as_i64().unwrap_or(i64::MIN);
            if idx >= i as i64 {
                return bad(
                    &format!("item {i}: reply_to index {idx} must reference an earlier item"),
                    "INVALID_REPLY_TO",
                );
            }
        }
        if let Err(resp) = validate_item(item) {
            return resp;
        }
        // caption length: reuse the 499-char guard
        if let Some(ref c) = item.caption {
            if c.chars().count() > 499 {
                return bad(
                    &format!(
                        "item {i}: caption exceeds the 499-character limit (got {})",
                        c.chars().count()
                    ),
                    "CAPTION_TOO_LONG",
                );
            }
        }
    }

    // Resolve mode + timestamp
    let (scheduled_at, root_status) = match input.scheduled_at.as_deref() {
        Some(sa) => {
            if parse_utc(sa).is_none() {
                return bad(
                    &format!(
                        "scheduled_at must be an ISO 8601 / RFC 3339 timestamp with offset, got: {sa}"
                    ),
                    "INVALID_SCHEDULED_AT",
                );
            }
            (
                sa.to_string(),
                if input.auto_approve {
                    "pending"
                } else {
                    "draft"
                },
            )
        }
        None => {
            // Instant mode: everything is pending immediately. Give each item
            // its slot = now (root) — chain promotion handles the order.
            (chrono::Utc::now().to_rfc3339(), "pending")
        }
    };

    let bundle_id = format!("bundle_{}", Uuid::now_v7());
    let mut created = Vec::with_capacity(n_items);
    let mut failures = Vec::new();

    for (i, item) in input.posts.iter().enumerate() {
        // Reply target: index -> defer via reply_to_seq marker. The scheduler
        // resolves indexes at publish time (the previous item's real post id
        // only exists after it publishes). External string ids pass through.
        let (reply_to_id, reply_to_seq) = match &item.reply_to {
            Some(serde_json::Value::Number(num)) => (None, num.as_i64()),
            Some(serde_json::Value::String(s)) if !s.trim().is_empty() => {
                (Some(s.trim().to_string()), None)
            }
            _ => (None, None),
        };

        let status = if i == 0 {
            root_status.to_string()
        } else {
            // Non-root items wait until their predecessor publishes.
            // In instant mode the root is already pending, so the first tick
            // publishes it and chain-promotes item 1 immediately.
            "bundle_waiting".to_string()
        };

        let cs = CreateSchedule {
            account_id: input.account_id.clone(),
            media_type: Some(
                item.media_type
                    .clone()
                    .unwrap_or_else(|| "TEXT".to_string()),
            ),
            caption: item.caption.clone(),
            text_attachment: None,
            media_urls: item.media_urls.clone(),
            // Waiting items get the bundle slot time; promotion overrides order.
            scheduled_at: scheduled_at.clone(),
            location_id: None,
            auto_approve: matches!(status.as_str(), "pending"),
            reply_to_id,
            bundle_id: Some(bundle_id.clone()),
            bundle_seq: Some(i as i64),
            bundle_total: Some(n_items as i64),
        };
        // reply_to_seq marker: encode into result_json-free dedicated place —
        // reuse reply_to_id with a marker prefix the scheduler understands.
        let mut cs = cs;
        if let Some(seq) = reply_to_seq {
            cs.reply_to_id = Some(format!("bundle:{bundle_id}:{seq}"));
        }
        // Force non-root items to bundle_waiting regardless of auto_approve.
        if i > 0 {
            cs.auto_approve = false;
        }

        let id = Uuid::now_v7().to_string();
        match state.store.create_schedule(&id, &cs).await {
            Ok(schedule) => {
                // create_schedule only writes 'draft'/'pending'. Non-root
                // items must be 'bundle_waiting': fix THIS row up immediately,
                // scoped to its id (a brand-new row created moments ago with
                // scheduled_at in the future cannot be claimed by a concurrent
                // tick, and the update touches nothing else — no bulk flip,
                // no root restore, no race).
                if i > 0 {
                    if let Err(e) = state
                        .store
                        .update_schedule_status(&id, "bundle_waiting", None, None)
                        .await
                    {
                        let msg = format!("failed to set bundle_waiting: {e}");
                        // This row is stuck as draft/pending and would
                        // duplicate the chain — mark it failed explicitly.
                        let _ = state
                            .store
                            .update_schedule_status(&id, "failed", None, Some(&msg))
                            .await;
                        failures.push(serde_json::json!({ "seq": i, "error": msg }));
                        // A gap at i strands everything after it (promotion is
                        // seq-1 -> seq): stop and fail the waiting members.
                        let _ = state
                            .store
                            .fail_remaining_bundle(
                                &bundle_id,
                                &format!("bundle creation aborted at seq {i}"),
                            )
                            .await;
                        continue;
                    }
                }
                created.push(serde_json::json!({
                    "id": schedule.id,
                    "seq": i,
                    "status": if i == 0 { root_status } else { "bundle_waiting" },
                }))
            }
            Err(e) => {
                failures.push(serde_json::json!({ "seq": i, "error": e.to_string() }));
                // A gap at i would strand every later item: stop creating and
                // fail the waiting members created so far.
                let _ = state
                    .store
                    .fail_remaining_bundle(
                        &bundle_id,
                        &format!("bundle creation aborted at seq {i}: {e}"),
                    )
                    .await;
                break;
            }
        }
    }

    let status = if failures.is_empty() {
        StatusCode::CREATED
    } else {
        StatusCode::MULTI_STATUS
    };

    (
        status,
        Json(serde_json::json!({
            "data": {
                "bundle_id": bundle_id,
                "items": created,
                "failures": failures,
                "mode": if input.scheduled_at.is_some() { "scheduled" } else { "instant" },
            }
        })),
    )
}

/// GET /api/threads/{bundle_id} — bundle overview (items in seq order).
pub async fn get_thread_bundle(
    State(state): State<crate::server::AppState>,
    axum::extract::Path(bundle_id): axum::extract::Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    match state.store.get_bundle_schedules(&bundle_id).await {
        Ok(items) if items.is_empty() => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "bundle not found", "code": "BUNDLE_NOT_FOUND" })),
        ),
        Ok(items) => {
            let published = state
                .store
                .count_bundle_status(&bundle_id, "published")
                .await
                .unwrap_or(0);
            let failed = state
                .store
                .count_bundle_status(&bundle_id, "failed")
                .await
                .unwrap_or(0);
            let total = items.len() as i64;
            let bundle_status = if failed > 0 {
                "failed"
            } else if published == total {
                "published"
            } else if published > 0 {
                "partial"
            } else {
                "queued"
            };
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "data": {
                        "bundle_id": bundle_id,
                        "status": bundle_status,
                        "published": published,
                        "failed": failed,
                        "total": total,
                        "items": items,
                    }
                })),
            )
        }
        // DB error is a server fault — 500, not 404.
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string(), "code": "INTERNAL" })),
        ),
    }
}
