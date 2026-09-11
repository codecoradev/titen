mod common;

use axum::body::Body;
use common::{body_to_json, send, test_app, test_pool, test_state};
use titen_core::Store;

async fn seed_comment(pool: &sqlx::SqlitePool, comment_id: &str) {
    let store = Store::new(pool.clone());
    let (account, _) = store
        .upsert_account(&titen_core::models::CreateAccount {
            username: Some("deltest".to_string()),
            user_id: Some("user_del".to_string()),
            access_token: "FAKE_TEST_TOKEN".to_string(),
            expires_at: "2099-12-31T00:00:00Z".to_string(),
            app_id: None,
            app_secret: None,
        })
        .await
        .expect("seed account");

    store
        .create_post(
            "post-del",
            &titen_core::models::CreatePost {
                account_id: account.id.clone(),
                media_type: Some("TEXT".to_string()),
                caption: Some("delete endpoint test".to_string()),
                text_attachment: None,
                image_url: None,
                video_url: None,
                image_urls: None,
                media_ids: None,
                alt_text: None,
                reply_to_id: None,
            },
        )
        .await
        .expect("seed post");

    store
        .insert_comment(
            comment_id,
            "post-del",
            Some("tc-del-1"),
            Some("someone"),
            None,
            "comment to delete",
        )
        .await
        .expect("seed comment");
}

#[tokio::test]
async fn delete_comment_returns_204_and_removes_row() {
    let pool = test_pool().await;
    let state = test_state(pool.clone());
    let app = test_app(state);
    seed_comment(&pool, "c-route-del").await;

    let req = axum::http::Request::builder()
        .method("DELETE")
        .uri("/api/comments/c-route-del")
        .body(Body::empty())
        .unwrap();
    let resp = send(req, &app).await;
    assert_eq!(resp.status(), 204);

    let store = Store::new(pool.clone());
    assert!(
        store.get_comment("c-route-del").await.is_err(),
        "row must be gone from the store"
    );
}

#[tokio::test]
async fn delete_missing_comment_returns_404() {
    let pool = test_pool().await;
    let state = test_state(pool.clone());
    let app = test_app(state);

    let req = axum::http::Request::builder()
        .method("DELETE")
        .uri("/api/comments/does-not-exist")
        .body(Body::empty())
        .unwrap();
    let resp = send(req, &app).await;
    assert_eq!(resp.status(), 404);

    let body = body_to_json(resp).await;
    assert_eq!(body["code"], "COMMENT_NOT_FOUND");
}
