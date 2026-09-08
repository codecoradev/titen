mod common;

use axum::body::Body;
use common::{body_to_json, create_test_account, send, test_app, test_pool, test_state};
use serde_json::json;

#[tokio::test]
async fn create_schedule() {
    let pool = test_pool().await;
    let state = test_state(pool.clone());
    let app = test_app(state);

    let account = create_test_account(&app, &pool).await;
    let account_id = account["id"].as_str().unwrap();

    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/schedules")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "account_id": account_id,
                "media_type": "TEXT",
                "caption": "Scheduled hello",
                "scheduled_at": "2099-06-15T12:00:00Z"
            }))
            .unwrap(),
        ))
        .unwrap();
    let resp = send(req, &app).await;
    assert_eq!(resp.status(), 201);

    let body = body_to_json(resp).await;
    assert_eq!(body["data"]["account_id"], account_id);
    assert_eq!(body["data"]["caption"], "Scheduled hello");
    assert_eq!(body["data"]["status"], "draft");
    assert!(!body["data"]["id"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn create_schedule_accepts_499_char_caption() {
    let pool = test_pool().await;
    let state = test_state(pool.clone());
    let app = test_app(state);

    let account = create_test_account(&app, &pool).await;
    let account_id = account["id"].as_str().unwrap();

    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/schedules")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "account_id": account_id,
                "media_type": "TEXT",
                "caption": "x".repeat(499),
                "scheduled_at": "2099-06-15T12:00:00Z"
            }))
            .unwrap(),
        ))
        .unwrap();
    let resp = send(req, &app).await;
    assert_eq!(resp.status(), 201);
}

#[tokio::test]
async fn create_schedule_rejects_500_char_caption() {
    let pool = test_pool().await;
    let state = test_state(pool.clone());
    let app = test_app(state);

    let account = create_test_account(&app, &pool).await;
    let account_id = account["id"].as_str().unwrap();

    // Titen enforces a 499-char guard: exactly 500 is already rejected so a
    // merged caption+link can never push past the Threads limit unseen.
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/schedules")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "account_id": account_id,
                "media_type": "TEXT",
                "caption": "x".repeat(500),
                "scheduled_at": "2099-06-15T12:00:00Z"
            }))
            .unwrap(),
        ))
        .unwrap();
    let resp = send(req, &app).await;
    assert_eq!(resp.status(), 400);
    let body = body_to_json(resp).await;
    assert_eq!(body["code"], "CAPTION_TOO_LONG");
}

#[tokio::test]
async fn create_schedule_rejects_unparseable_scheduled_at() {
    let pool = test_pool().await;
    let state = test_state(pool.clone());
    let app = test_app(state);

    let account = create_test_account(&app, &pool).await;
    let account_id = account["id"].as_str().unwrap();

    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/schedules")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "account_id": account_id,
                "media_type": "TEXT",
                "caption": "hello",
                "scheduled_at": "sometime next week"
            }))
            .unwrap(),
        ))
        .unwrap();
    let resp = send(req, &app).await;
    assert_eq!(resp.status(), 400);
    let body = body_to_json(resp).await;
    assert_eq!(body["code"], "INVALID_SCHEDULED_AT");
}

#[tokio::test]
async fn create_schedule_accepts_offset_timestamps() {
    let pool = test_pool().await;
    let state = test_state(pool.clone());
    let app = test_app(state);

    let account = create_test_account(&app, &pool).await;
    let account_id = account["id"].as_str().unwrap();

    // +07:00 offsets (WIB) are the common authoring format — must parse.
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/schedules")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "account_id": account_id,
                "media_type": "TEXT",
                "caption": "hello",
                "scheduled_at": "2099-06-15T12:00:00+07:00"
            }))
            .unwrap(),
        ))
        .unwrap();
    let resp = send(req, &app).await;
    assert_eq!(resp.status(), 201);
}

#[tokio::test]
async fn list_schedules() {
    let pool = test_pool().await;
    let state = test_state(pool.clone());
    let app = test_app(state);

    let account = create_test_account(&app, &pool).await;
    let account_id = account["id"].as_str().unwrap();

    let store = titen_core::Store::new(pool.clone());
    for i in 0..2 {
        let id = uuid::Uuid::now_v7().to_string();
        store
            .create_schedule(
                &id,
                &titen_core::models::CreateSchedule {
                    account_id: account_id.to_string(),
                    media_type: Some("TEXT".to_string()),
                    caption: Some(format!("Scheduled {i}")),
                    text_attachment: None,
                    media_urls: None,
                    scheduled_at: format!("2099-07-{:02}T12:00:00Z", i + 10),
                    location_id: None,
                    auto_approve: true,
                    reply_to_id: None,
                },
            )
            .await
            .expect("Failed to create schedule via store");
    }

    let req = axum::http::Request::builder()
        .method("GET")
        .uri("/api/schedules")
        .body(Body::empty())
        .unwrap();
    let resp = send(req, &app).await;
    assert_eq!(resp.status(), 200);

    let body = body_to_json(resp).await;
    let schedules = body["data"].as_array().unwrap();
    assert_eq!(schedules.len(), 2);
}

#[tokio::test]
async fn list_schedules_filters_by_account() {
    let pool = test_pool().await;
    let state = test_state(pool.clone());
    let app = test_app(state);

    let account = create_test_account(&app, &pool).await;
    let account_id = account["id"].as_str().unwrap();

    let store = titen_core::Store::new(pool.clone());
    let id = uuid::Uuid::now_v7().to_string();
    store
        .create_schedule(
            &id,
            &titen_core::models::CreateSchedule {
                account_id: account_id.to_string(),
                media_type: Some("TEXT".to_string()),
                caption: Some("Filtered schedule".to_string()),
                text_attachment: None,
                media_urls: None,
                scheduled_at: "2099-08-01T12:00:00Z".to_string(),
                location_id: None,
                auto_approve: true,
                reply_to_id: None,
            },
        )
        .await
        .expect("Failed to create schedule via store");

    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/api/schedules?account_id={account_id}"))
        .body(Body::empty())
        .unwrap();
    let resp = send(req, &app).await;
    assert_eq!(resp.status(), 200);

    let body = body_to_json(resp).await;
    let schedules = body["data"].as_array().unwrap();
    assert_eq!(schedules.len(), 1);
    assert_eq!(schedules[0]["caption"], "Filtered schedule");
}

#[tokio::test]
async fn cancel_delete_schedule() {
    let pool = test_pool().await;
    let state = test_state(pool.clone());
    let app = test_app(state);

    let account = create_test_account(&app, &pool).await;
    let account_id = account["id"].as_str().unwrap();

    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/schedules")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "account_id": account_id,
                "media_type": "TEXT",
                "caption": "Will be cancelled",
                "scheduled_at": "2099-09-01T12:00:00Z"
            }))
            .unwrap(),
        ))
        .unwrap();
    let resp = send(req, &app).await;
    assert_eq!(resp.status(), 201);

    let body = body_to_json(resp).await;
    let schedule_id = body["data"]["id"].as_str().unwrap().to_string();

    let req2 = axum::http::Request::builder()
        .method("DELETE")
        .uri(format!("/api/schedules/{schedule_id}"))
        .body(Body::empty())
        .unwrap();
    let resp2 = send(req2, &app).await;
    assert_eq!(resp2.status(), 200);

    let body2 = body_to_json(resp2).await;
    assert_eq!(body2["data"], json!(null));

    let req3 = axum::http::Request::builder()
        .method("GET")
        .uri("/api/schedules")
        .body(Body::empty())
        .unwrap();
    let resp3 = send(req3, &app).await;
    let body3 = body_to_json(resp3).await;
    assert_eq!(body3["data"], json!([]));
}

#[tokio::test]
async fn list_schedules_pagination_meta() {
    let pool = test_pool().await;
    let state = test_state(pool.clone());
    let app = test_app(state);

    let account = create_test_account(&app, &pool).await;
    let account_id = account["id"].as_str().unwrap();

    let store = titen_core::Store::new(pool.clone());
    for i in 0..5 {
        let id = uuid::Uuid::now_v7().to_string();
        store
            .create_schedule(
                &id,
                &titen_core::models::CreateSchedule {
                    account_id: account_id.to_string(),
                    media_type: Some("TEXT".to_string()),
                    caption: Some(format!("Paged {i}")),
                    text_attachment: None,
                    media_urls: None,
                    scheduled_at: format!("2099-08-{:02}T09:00:00Z", i + 1),
                    location_id: None,
                    auto_approve: true,
                    reply_to_id: None,
                },
            )
            .await
            .expect("Failed to create schedule via store");
    }

    // Page 1: first 2 rows + total
    let req = axum::http::Request::builder()
        .method("GET")
        .uri("/api/schedules?limit=2&offset=0")
        .body(Body::empty())
        .unwrap();
    let resp = send(req, &app).await;
    assert_eq!(resp.status(), 200);
    let body = body_to_json(resp).await;
    assert_eq!(body["data"].as_array().unwrap().len(), 2);
    assert_eq!(body["total"], 5);
    assert_eq!(body["limit"], 2);
    assert_eq!(body["offset"], 0);

    // Page 2: rows 3-4, no overlap with page 1
    let req = axum::http::Request::builder()
        .method("GET")
        .uri("/api/schedules?limit=2&offset=2")
        .body(Body::empty())
        .unwrap();
    let resp = send(req, &app).await;
    let body2 = body_to_json(resp).await;
    assert_eq!(body2["data"].as_array().unwrap().len(), 2);
    assert_eq!(body2["total"], 5);

    let ids1: Vec<&str> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap())
        .collect();
    let ids2: Vec<&str> = body2["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap())
        .collect();
    assert!(
        ids1.iter().all(|id| !ids2.contains(id)),
        "pages must not overlap"
    );

    // Backward-compat: `data` remains a plain array
    assert!(body["data"].is_array());
}

// ─── #232: schedulable thread replies (reply_to_id) ─────────────────────

#[tokio::test]
async fn create_reply_schedule_persists_reply_to_id() {
    let pool = test_pool().await;
    let state = test_state(pool.clone());
    let app = test_app(state);

    let account = create_test_account(&app, &pool).await;
    let account_id = account["id"].as_str().unwrap();

    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/schedules")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "account_id": account_id,
                "media_type": "TEXT",
                "caption": "Scheduled reply",
                "scheduled_at": "2099-06-15T13:00:00Z",
                "reply_to_id": "12345678901234567"
            }))
            .unwrap(),
        ))
        .unwrap();
    let resp = send(req, &app).await;
    assert_eq!(resp.status(), 201);

    let body = body_to_json(resp).await;
    assert_eq!(body["data"]["reply_to_id"], "12345678901234567");
    assert_eq!(body["data"]["status"], "draft");
}

#[tokio::test]
async fn create_reply_schedule_rejects_non_text() {
    let pool = test_pool().await;
    let state = test_state(pool.clone());
    let app = test_app(state);

    let account = create_test_account(&app, &pool).await;
    let account_id = account["id"].as_str().unwrap();

    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/schedules")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "account_id": account_id,
                "media_type": "IMAGE",
                "caption": "img reply",
                "scheduled_at": "2099-06-15T13:00:00Z",
                "reply_to_id": "12345678901234567"
            }))
            .unwrap(),
        ))
        .unwrap();
    let resp = send(req, &app).await;
    assert_eq!(resp.status(), 400);

    let body = body_to_json(resp).await;
    assert_eq!(body["code"], "INVALID_REPLY_TO_ID");
}

#[tokio::test]
async fn create_reply_schedule_rejects_empty_target() {
    let pool = test_pool().await;
    let state = test_state(pool.clone());
    let app = test_app(state);

    let account = create_test_account(&app, &pool).await;
    let account_id = account["id"].as_str().unwrap();

    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/schedules")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "account_id": account_id,
                "media_type": "TEXT",
                "caption": "blank target",
                "scheduled_at": "2099-06-15T13:00:00Z",
                "reply_to_id": "   "
            }))
            .unwrap(),
        ))
        .unwrap();
    let resp = send(req, &app).await;
    assert_eq!(resp.status(), 400);
    let body = body_to_json(resp).await;
    assert_eq!(body["code"], "INVALID_REPLY_TO_ID");
}

#[tokio::test]
async fn patch_draft_schedule_updates_reply_to_id() {
    let pool = test_pool().await;
    let state = test_state(pool.clone());
    let app = test_app(state);

    let account = create_test_account(&app, &pool).await;
    let account_id = account["id"].as_str().unwrap();

    // Create a draft
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/schedules")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "account_id": account_id,
                "media_type": "TEXT",
                "caption": "draft to patch",
                "scheduled_at": "2099-06-15T14:00:00Z"
            }))
            .unwrap(),
        ))
        .unwrap();
    let resp = send(req, &app).await;
    assert_eq!(resp.status(), 201);
    let body = body_to_json(resp).await;
    let schedule_id = body["data"]["id"].as_str().unwrap().to_string();

    // PATCH reply_to_id onto the draft
    let req = axum::http::Request::builder()
        .method("PATCH")
        .uri(format!("/api/schedules/{schedule_id}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "reply_to_id": "98765432109876543"
            }))
            .unwrap(),
        ))
        .unwrap();
    let resp = send(req, &app).await;
    assert_eq!(resp.status(), 200);

    let body = body_to_json(resp).await;
    assert_eq!(body["data"]["reply_to_id"], "98765432109876543");
}

// ─── #242: agent/CI ingest endpoint ─────────────────────────────────────

#[tokio::test]
async fn ingest_creates_draft_with_source() {
    let pool = test_pool().await;
    let state = test_state(pool.clone());
    let app = test_app(state);

    let account = create_test_account(&app, &pool).await;
    let account_id = account["id"].as_str().unwrap();

    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/schedules/ingest")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "account_id": account_id,
                "media_type": "TEXT",
                "caption": "Agent draft post",
                "scheduled_at": "2099-07-01T09:00:00Z",
                "source": "cmo-agent"
            }))
            .unwrap(),
        ))
        .unwrap();
    let resp = send(req, &app).await;
    assert_eq!(resp.status(), 201);

    let body = body_to_json(resp).await;
    assert_eq!(body["data"]["status"], "draft");
    assert_eq!(body["data"]["source"], "cmo-agent");
}

#[tokio::test]
async fn ingest_rejects_caption_over_limit() {
    let pool = test_pool().await;
    let state = test_state(pool.clone());
    let app = test_app(state);

    let account = create_test_account(&app, &pool).await;
    let account_id = account["id"].as_str().unwrap();

    let long_caption = "x".repeat(501);
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/schedules/ingest")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "account_id": account_id,
                "media_type": "TEXT",
                "caption": long_caption,
                "scheduled_at": "2099-07-01T09:00:00Z"
            }))
            .unwrap(),
        ))
        .unwrap();
    let resp = send(req, &app).await;
    assert_eq!(resp.status(), 400);
    let body = body_to_json(resp).await;
    assert_eq!(body["code"], "CAPTION_TOO_LONG");
}

#[tokio::test]
async fn ingest_rejects_invalid_reply_target() {
    let pool = test_pool().await;
    let state = test_state(pool.clone());
    let app = test_app(state);

    let account = create_test_account(&app, &pool).await;
    let account_id = account["id"].as_str().unwrap();

    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/schedules/ingest")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "account_id": account_id,
                "media_type": "IMAGE",
                "caption": "bad reply",
                "scheduled_at": "2099-07-01T09:00:00Z",
                "reply_to_id": "123456789"
            }))
            .unwrap(),
        ))
        .unwrap();
    let resp = send(req, &app).await;
    assert_eq!(resp.status(), 400);
    let body = body_to_json(resp).await;
    assert_eq!(body["code"], "INVALID_REPLY_TO_ID");
}
