mod common;

use axum::body::Body;
use common::{body_to_json, create_test_account, send, test_app, test_pool, test_state};
use serde_json::json;

#[tokio::test]
async fn list_accounts_empty() {
    let pool = test_pool().await;
    let state = test_state(pool);
    let app = test_app(state);

    let req = axum::http::Request::builder()
        .method("GET")
        .uri("/api/accounts")
        .body(Body::empty())
        .unwrap();
    let resp = send(req, &app).await;
    assert_eq!(resp.status(), 200);

    let body = body_to_json(resp).await;
    assert_eq!(body["data"], json!([]));
}

#[tokio::test]
async fn create_account() {
    let pool = test_pool().await;
    let state = test_state(pool);
    let app = test_app(state);

    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/accounts")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "username": "alice",
                "user_id": "usr_001",
                "access_token": "tok_abc",
                "expires_at": "2099-12-31T00:00:00Z",
                "refresh_token": "ref_abc"
            }))
            .unwrap(),
        ))
        .unwrap();
    let resp = send(req, &app).await;
    assert_eq!(resp.status(), 201);

    let body = body_to_json(resp).await;
    assert_eq!(body["data"]["username"], "alice");
    assert_eq!(body["data"]["user_id"], "usr_001");
    assert_eq!(body["data"]["is_active"], true);
    assert!(!body["data"]["id"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn get_account_by_id() {
    let pool = test_pool().await;
    let state = test_state(pool.clone());
    let app = test_app(state);

    let account = create_test_account(&app, &pool).await;
    let id = account["id"].as_str().unwrap();

    let req = axum::http::Request::builder()
        .method("GET")
        .uri("/api/accounts")
        .body(Body::empty())
        .unwrap();
    let resp = send(req, &app).await;
    let body = body_to_json(resp).await;

    let found = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == id);
    assert!(found.is_some());
    assert_eq!(found.unwrap()["username"], "testuser");
}

#[tokio::test]
async fn get_account_by_username() {
    let pool = test_pool().await;
    let state = test_state(pool.clone());
    let app = test_app(state);

    let account = create_test_account(&app, &pool).await;

    let req = axum::http::Request::builder()
        .method("GET")
        .uri("/api/accounts")
        .body(Body::empty())
        .unwrap();
    let resp = send(req, &app).await;
    let body = body_to_json(resp).await;

    let found = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["username"] == "testuser");
    assert!(found.is_some());
    assert_eq!(found.unwrap()["user_id"], "user_123");
}

#[tokio::test]
async fn update_account() {
    let pool = test_pool().await;
    let state = test_state(pool.clone());
    let app = test_app(state);

    let account = create_test_account(&app, &pool).await;
    let id = account["id"].as_str().unwrap();

    let req = axum::http::Request::builder()
        .method("PUT")
        .uri(format!("/api/accounts/{id}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "access_token": "new_token_xyz",
                "expires_at": "2099-06-01T00:00:00Z",
                "is_active": false
            }))
            .unwrap(),
        ))
        .unwrap();
    let resp = send(req, &app).await;
    assert_eq!(resp.status(), 200);

    let body = body_to_json(resp).await;
    assert_eq!(body["data"]["is_active"], false);

    let req2 = axum::http::Request::builder()
        .method("GET")
        .uri("/api/accounts")
        .body(Body::empty())
        .unwrap();
    let resp2 = send(req2, &app).await;
    let body2 = body_to_json(resp2).await;
    let found = body2["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == id);
    assert_eq!(found.unwrap()["is_active"], false);
}

#[tokio::test]
async fn delete_account() {
    let pool = test_pool().await;
    let state = test_state(pool.clone());
    let app = test_app(state);

    let account = create_test_account(&app, &pool).await;
    let id = account["id"].as_str().unwrap();

    let req = axum::http::Request::builder()
        .method("DELETE")
        .uri(format!("/api/accounts/{id}"))
        .body(Body::empty())
        .unwrap();
    let resp = send(req, &app).await;
    assert_eq!(resp.status(), 200);

    let body = body_to_json(resp).await;
    assert_eq!(body["data"], json!(null));

    let req2 = axum::http::Request::builder()
        .method("GET")
        .uri("/api/accounts")
        .body(Body::empty())
        .unwrap();
    let resp2 = send(req2, &app).await;
    let body2 = body_to_json(resp2).await;
    assert_eq!(body2["data"], json!([]));
}

#[tokio::test]
async fn duplicate_account_error() {
    let pool = test_pool().await;
    let state = test_state(pool);
    let app = test_app(state);

    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/accounts")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "username": "dup_user",
                "user_id": "usr_dup",
                "access_token": "tok_dup",
                "expires_at": "2099-12-31T00:00:00Z",
            }))
            .unwrap(),
        ))
        .unwrap();
    let resp = send(req, &app).await;
    assert_eq!(resp.status(), 201);

    let req2 = axum::http::Request::builder()
        .method("POST")
        .uri("/api/accounts")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "username": "dup_user",
                "user_id": "usr_dup2",
                "access_token": "tok_dup2",
                "expires_at": "2099-12-31T00:00:00Z",
            }))
            .unwrap(),
        ))
        .unwrap();
    let resp2 = send(req2, &app).await;
    assert_eq!(resp2.status(), 409);

    let body = body_to_json(resp2).await;
    assert!(body["error"].is_string());
    assert!(body["code"] == "CREATE_FAILED");
}
