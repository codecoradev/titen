mod common;

use axum::body::Body;
use common::{body_to_json, create_test_account, send, test_app, test_pool, test_state};
use tower::ServiceExt;

#[tokio::test]
async fn create_and_list_posts() {
    let pool = test_pool().await;
    let state = test_state(pool.clone());
    let app = test_app(state);

    let account = create_test_account(&app, &pool).await;
    let account_id = account["id"].as_str().unwrap();

    let store = titen_core::Store::new(pool.clone());
    let post_id = uuid::Uuid::now_v7().to_string();
    let post = store
        .create_post(
            &post_id,
            &titen_core::models::CreatePost {
                account_id: account_id.to_string(),
                media_type: Some("TEXT".to_string()),
                caption: Some("Hello, Threads!".to_string()),
                text_attachment: None,
                image_url: None,
                video_url: None,
                image_urls: None,
                alt_text: None,
            },
        )
        .await
        .expect("Failed to create post via store");

    let req = axum::http::Request::builder()
        .method("GET")
        .uri("/api/posts")
        .body(Body::empty())
        .unwrap();
    let resp = send(req, &app).await;
    assert_eq!(resp.status(), 200);

    let body = body_to_json(resp).await;
    let posts = body["data"].as_array().unwrap();
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0]["id"], post.id);
    assert_eq!(posts[0]["caption"], "Hello, Threads!");
}

#[tokio::test]
async fn get_post_by_id() {
    let pool = test_pool().await;
    let state = test_state(pool.clone());
    let app = test_app(state);

    let account = create_test_account(&app, &pool).await;
    let account_id = account["id"].as_str().unwrap();

    let store = titen_core::Store::new(pool.clone());
    let post_id = uuid::Uuid::now_v7().to_string();
    store
        .create_post(
            &post_id,
            &titen_core::models::CreatePost {
                account_id: account_id.to_string(),
                media_type: Some("TEXT".to_string()),
                caption: Some("Test post content".to_string()),
                text_attachment: None,
                image_url: None,
                video_url: None,
                image_urls: None,
                alt_text: None,
            },
        )
        .await
        .expect("Failed to create post via store");

    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/api/posts/{post_id}"))
        .body(Body::empty())
        .unwrap();
    let resp = send(req, &app).await;
    assert_eq!(resp.status(), 200);

    let body = body_to_json(resp).await;
    assert_eq!(body["data"]["id"], post_id);
    assert_eq!(body["data"]["caption"], "Test post content");
    assert_eq!(body["data"]["account_id"], account_id);
}

#[tokio::test]
async fn delete_post() {
    let pool = test_pool().await;
    let state = test_state(pool.clone());
    let app = test_app(state);

    let account = create_test_account(&app, &pool).await;
    let account_id = account["id"].as_str().unwrap();

    let store = titen_core::Store::new(pool.clone());
    let post_id = uuid::Uuid::now_v7().to_string();
    store
        .create_post(
            &post_id,
            &titen_core::models::CreatePost {
                account_id: account_id.to_string(),
                media_type: Some("TEXT".to_string()),
                caption: Some("To be deleted".to_string()),
                text_attachment: None,
                image_url: None,
                video_url: None,
                image_urls: None,
                alt_text: None,
            },
        )
        .await
        .expect("Failed to create post via store");

    let req = axum::http::Request::builder()
        .method("DELETE")
        .uri(format!("/api/posts/{post_id}"))
        .body(Body::empty())
        .unwrap();
    let resp = send(req, &app).await;
    assert_eq!(resp.status(), 200);

    let body = body_to_json(resp).await;
    assert_eq!(body["data"], serde_json::json!(null));

    let req2 = axum::http::Request::builder()
        .method("GET")
        .uri("/api/posts")
        .body(Body::empty())
        .unwrap();
    let resp2 = send(req2, &app).await;
    let body2 = body_to_json(resp2).await;
    assert_eq!(body2["data"], serde_json::json!([]));
}

#[tokio::test]
async fn list_posts_filters_by_account() {
    let pool = test_pool().await;
    let state = test_state(pool.clone());
    let app = test_app(state);

    let account = create_test_account(&app, &pool).await;
    let account_id = account["id"].as_str().unwrap();

    let store = titen_core::Store::new(pool.clone());
    for i in 0..2 {
        let post_id = uuid::Uuid::now_v7().to_string();
        store
            .create_post(
                &post_id,
                &titen_core::models::CreatePost {
                    account_id: account_id.to_string(),
                    media_type: Some("TEXT".to_string()),
                    caption: Some(format!("Post {i}")),
                    text_attachment: None,
                    image_url: None,
                    video_url: None,
                    image_urls: None,
                    alt_text: None,
                },
            )
            .await
            .expect("Failed to create post via store");
    }

    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/api/posts?account_id={account_id}"))
        .body(Body::empty())
        .unwrap();
    let resp = send(req, &app).await;
    assert_eq!(resp.status(), 200);

    let body = body_to_json(resp).await;
    let posts = body["data"].as_array().unwrap();
    assert_eq!(posts.len(), 2);
}
