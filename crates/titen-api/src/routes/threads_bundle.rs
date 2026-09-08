//! Thread bundles (#thread-bundle Phase 2): one root post plus chained
//! replies, publishable instantly or as a single scheduled slot.

use axum::{Json, extract::State, http::StatusCode};
use uuid::Uuid;

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
        // alt_text: not plumbed through the schedule path yet — reject
        // explicitly instead of silently discarding accessibility text.
        if item
            .alt_text
            .as_deref()
            .map(str::trim)
            .map(str::len)
            .unwrap_or(0)
            > 0
        {
            return bad(
                &format!("item {i}: alt_text is not yet supported on bundles; omit it for now"),
                "ALT_TEXT_UNSUPPORTED",
            );
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

    // Single transaction: either the whole bundle exists or nothing does.
    // This kills every partial-state class (orphaned drafts, gaps in seq,
    // waiting rows stranded by a mid-loop failure).
    let mut tx = match state.store.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            return bad(
                &format!("failed to begin transaction: {e}"),
                "CREATE_FAILED",
            );
        }
    };

    let mut created = Vec::with_capacity(n_items);

    for (i, item) in input.posts.iter().enumerate() {
        // Reply target: index -> defer via bundle marker (the referenced
        // item's real post id only exists after it publishes). External
        // string ids pass through as-is.
        let reply_to_id = match &item.reply_to {
            Some(serde_json::Value::Number(num)) => {
                let seq = num.as_i64().unwrap_or(i64::MIN);
                Some(format!("bundle:{bundle_id}:{seq}"))
            }
            Some(serde_json::Value::String(s)) if !s.trim().is_empty() => {
                Some(s.trim().to_string())
            }
            _ => None,
        };

        let status = if i == 0 {
            root_status.to_string()
        } else {
            // Non-root items wait until their predecessor publishes. In
            // instant mode the root is pending, so the next tick publishes
            // it and chain-promotes item 1.
            "bundle_waiting".to_string()
        };

        let row = titen_core::models::CreateBundleItem {
            account_id: input.account_id.clone(),
            media_type: item
                .media_type
                .clone()
                .unwrap_or_else(|| "TEXT".to_string()),
            caption: item.caption.clone(),
            media_urls: item.media_urls.clone(),
            scheduled_at: scheduled_at.clone(),
            reply_to_id,
            bundle_id: bundle_id.clone(),
            bundle_seq: i as i64,
            bundle_total: n_items as i64,
            status: status.clone(),
        };

        let id = Uuid::now_v7().to_string();
        let create_result = state.store.create_bundle_item_tx(&mut tx, &id, &row).await;
        match create_result {
            Ok(schedule) => created.push(serde_json::json!({
                "id": schedule.id,
                "seq": i,
                "status": schedule.status,
            })),
            Err(e) => {
                // tx drops here → rollback: nothing partial survives.
                return bad(
                    &format!("failed to create item {i}: {e} — bundle rolled back"),
                    "CREATE_FAILED",
                );
            }
        }
    }

    if let Err(e) = tx.commit().await {
        return bad(&format!("failed to commit bundle: {e}"), "CREATE_FAILED");
    }

    let status = StatusCode::CREATED;

    (
        status,
        Json(serde_json::json!({
            "data": {
                "bundle_id": bundle_id,
                "items": created,
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
