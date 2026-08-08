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
                    auto_approve: true,
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
                auto_approve: true,
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
