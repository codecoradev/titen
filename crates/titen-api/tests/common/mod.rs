use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    routing::{delete, get, post, put},
};
use serde_json::Value;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use tower::ServiceExt;

use titen_api::routes;
use titen_api::server::AppState;
use titen_core::{Store, ThreadsClient};

/// Create an in-memory SQLite pool with max_connections(1) and run migrations.
pub async fn test_pool() -> SqlitePool {
    let options = SqliteConnectOptions::new()
        .filename(":memory:")
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1) // CRITICAL — single connection = single in-memory DB
        .connect_with(options)
        .await
        .expect("Failed to create in-memory SQLite pool");

    let store = Store::new(pool.clone());
    store.migrate().await.expect("Failed to run migrations");
    pool
}

/// Build the AppState with no API key (auth middleware allows all requests).
pub fn test_state(pool: SqlitePool) -> AppState {
    let store = Arc::new(Store::new(pool));
    let threads_client = Arc::new(ThreadsClient::new(store.clone()));
    AppState {
        store,
        threads_client,
        api_key: None,
    }
}

/// Build a test Router (no auth middleware, no tracing/cors layers).
pub fn test_app(state: AppState) -> Router {
    Router::new()
        .route(
            "/api/accounts",
            get(routes::accounts::list_accounts).post(routes::accounts::create_account),
        )
        .route(
            "/api/accounts/{id}",
            put(routes::accounts::update_account).delete(routes::accounts::delete_account),
        )
        .route(
            "/api/posts",
            get(routes::posts::list_posts).post(routes::posts::create_post),
        )
        .route(
            "/api/posts/{id}",
            get(routes::posts::get_post).delete(routes::posts::delete_post),
        )
        .route(
            "/api/schedules",
            get(routes::schedules::list_schedules).post(routes::schedules::create_schedule),
        )
        .route(
            "/api/schedules/{id}",
            put(routes::schedules::update_schedule).delete(routes::schedules::delete_schedule),
        )
        .route(
            "/api/schedules/upcoming",
            get(routes::schedules::list_upcoming),
        )
        .with_state(state)
}

/// Send a request to a cloned Router and return the response.
/// Use this for making multiple requests from the same router.
pub async fn send(req: axum::http::Request<Body>, app: &Router) -> axum::http::Response<Body> {
    app.clone().oneshot(req).await.unwrap()
}

/// Helper: create a test account via the Store directly (avoids Threads API calls).
/// Returns the account JSON from the list endpoint.
pub async fn create_test_account(app: &Router, pool: &SqlitePool) -> Value {
    let store = Store::new(pool.clone());
    let id = uuid::Uuid::now_v7().to_string();
    let input = titen_core::models::CreateAccount {
        username: "testuser".to_string(),
        user_id: "user_123".to_string(),
        access_token: "fake_token".to_string(),
        expires_at: "2099-12-31T00:00:00Z".to_string(),
        refresh_token: Some("fake_refresh".to_string()),
        app_id: None,
    };
    store
        .create_account(&id, &input)
        .await
        .expect("Failed to create test account");

    // Fetch back via API to get the JSON shape
    let req = axum::http::Request::builder()
        .method("GET")
        .uri("/api/accounts")
        .body(Body::empty())
        .unwrap();
    let resp = send(req, app).await;
    let body = body_to_json(resp).await;
    body["data"].as_array().unwrap()[0].clone()
}

/// Parse a response body into serde_json::Value.
pub async fn body_to_json(resp: axum::http::Response<Body>) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    serde_json::from_slice(&bytes).expect("Failed to parse response as JSON")
}
