//! Integration tests for titen-mcp tool handlers.
//!
//! Since `handle_tool_call` is a private function requiring a `&tokio::runtime::Runtime`
//! and live Threads API client, these integration tests exercise the same `Store`
//! operations that the MCP handlers wrap. This validates the data layer correctness
//! for all MCP tool use cases.

use titen_core::Store;
use titen_core::models::{
    CommentFilter, CreateAccount, CreatePost, CreateSchedule, MediaFilter, MentionFilter,
    PostFilter, ScheduleFilter,
};

/// Helper: create an in-memory Store with migrations applied.
async fn setup_store() -> Store {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to connect to in-memory DB");
    let store = Store::new(pool);
    store.migrate().await.expect("Failed to run migrations");
    store
}

/// Helper: create a test account.
async fn create_test_account(store: &Store, id: &str, username: &str) {
    let input = CreateAccount {
        username: Some(username.to_string()),
        user_id: Some("threads_user_123".to_string()),
        access_token: "test_token_abc".to_string(),
        expires_at: "2026-12-31T23:59:59Z".to_string(),
        app_id: Some("test_app_id".to_string()),
        app_secret: None,
    };
    store
        .create_account(id, &input)
        .await
        .expect("Failed to create test account");
}

/// Helper: create a test post.
async fn create_test_post(store: &Store, id: &str, account_id: &str, caption: &str) {
    let input = CreatePost {
        account_id: account_id.to_string(),
        caption: Some(caption.to_string()),
        media_type: Some("TEXT".to_string()),
        image_url: None,
        text_attachment: None,
        video_url: None,
        image_urls: None,
        media_ids: None,
        alt_text: None,
    };
    store
        .create_post(id, &input)
        .await
        .expect("Failed to create test post");
}

/// Helper: create a test schedule.
async fn create_test_schedule(store: &Store, id: &str, account_id: &str) {
    let input = CreateSchedule {
        account_id: account_id.to_string(),
        caption: Some("Scheduled post caption".to_string()),
        media_type: Some("TEXT".to_string()),
        text_attachment: None,
        media_urls: None,
        scheduled_at: "2026-12-31T10:00:00Z".to_string(),
        location_id: None,
        auto_approve: true,
    };
    store
        .create_schedule(id, &input)
        .await
        .expect("Failed to create test schedule");
}

// ═══════════════════════════════════════════════════════════════
// Account operations (list_accounts tool)
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_list_accounts_empty_db() {
    let store = setup_store().await;
    let accounts = store.list_accounts().await.expect("list_accounts failed");
    assert!(accounts.is_empty(), "Empty DB should return no accounts");
}

#[tokio::test]
async fn test_create_account_then_list() {
    let store = setup_store().await;
    create_test_account(&store, "acc-1", "testuser").await;

    let accounts = store.list_accounts().await.expect("list_accounts failed");
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].id, "acc-1");
    assert_eq!(accounts[0].username, "testuser");
    assert!(accounts[0].is_active);
}

#[tokio::test]
async fn test_create_multiple_accounts() {
    let store = setup_store().await;
    create_test_account(&store, "acc-1", "user1").await;
    create_test_account(&store, "acc-2", "user2").await;
    create_test_account(&store, "acc-3", "user3").await;

    let accounts = store.list_accounts().await.expect("list_accounts failed");
    assert_eq!(accounts.len(), 3);
}

#[tokio::test]
async fn test_get_account_not_found() {
    let store = setup_store().await;
    let result = store.get_account("nonexistent").await;
    assert!(result.is_err(), "Getting nonexistent account should error");
}

// ═══════════════════════════════════════════════════════════════
// Post operations (create_post, list_posts, get_post, delete_post)
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_create_post_then_list() {
    let store = setup_store().await;
    create_test_account(&store, "acc-1", "testuser").await;
    create_test_post(&store, "post-1", "acc-1", "Hello world!").await;

    let posts = store
        .list_posts(&PostFilter::default())
        .await
        .expect("list_posts failed");
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0].id, "post-1");
    assert_eq!(posts[0].caption.as_deref(), Some("Hello world!"));
}

#[tokio::test]
async fn test_create_post_then_get() {
    let store = setup_store().await;
    create_test_account(&store, "acc-1", "testuser").await;
    create_test_post(&store, "post-1", "acc-1", "My caption").await;

    let post = store.get_post("post-1").await.expect("get_post failed");
    assert_eq!(post.id, "post-1");
    assert_eq!(post.account_id, "acc-1");
    assert_eq!(post.caption.as_deref(), Some("My caption"));
    assert_eq!(post.media_type, "TEXT");
}

#[tokio::test]
async fn test_list_posts_empty() {
    let store = setup_store().await;
    let posts = store
        .list_posts(&PostFilter::default())
        .await
        .expect("list_posts failed");
    assert!(posts.is_empty());
}

#[tokio::test]
async fn test_list_posts_filtered_by_account() {
    let store = setup_store().await;
    create_test_account(&store, "acc-1", "user1").await;
    create_test_account(&store, "acc-2", "user2").await;
    create_test_post(&store, "post-1", "acc-1", "from acc-1").await;
    create_test_post(&store, "post-2", "acc-2", "from acc-2").await;

    let filter = PostFilter {
        account_id: Some("acc-1".to_string()),
        ..Default::default()
    };
    let posts = store.list_posts(&filter).await.expect("list_posts failed");
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0].account_id, "acc-1");
}

#[tokio::test]
async fn test_delete_post_existing() {
    let store = setup_store().await;
    create_test_account(&store, "acc-1", "user1").await;
    create_test_post(&store, "post-1", "acc-1", "to delete").await;

    store
        .delete_post("post-1")
        .await
        .expect("delete_post should succeed");

    let result = store.get_post("post-1").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_delete_post_nonexistent_returns_error() {
    let store = setup_store().await;
    let result = store.delete_post("nonexistent-id").await;
    assert!(
        result.is_err(),
        "Deleting nonexistent post should return error"
    );
}

#[tokio::test]
async fn test_get_post_nonexistent_returns_error() {
    let store = setup_store().await;
    let result = store.get_post("nonexistent-id").await;
    assert!(result.is_err());
}

// ═══════════════════════════════════════════════════════════════
// Schedule operations (schedule_post, list_schedules, cancel_schedule)
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_create_schedule_then_list() {
    let store = setup_store().await;
    create_test_account(&store, "acc-1", "user1").await;
    create_test_schedule(&store, "sched-1", "acc-1").await;

    let schedules = store
        .list_schedules(&ScheduleFilter::default())
        .await
        .expect("list_schedules failed");
    assert_eq!(schedules.len(), 1);
    assert_eq!(schedules[0].id, "sched-1");
}

#[tokio::test]
async fn test_create_schedule_then_get() {
    let store = setup_store().await;
    create_test_account(&store, "acc-1", "user1").await;
    create_test_schedule(&store, "sched-1", "acc-1").await;

    let schedule = store
        .get_schedule("sched-1")
        .await
        .expect("get_schedule failed");
    assert_eq!(schedule.id, "sched-1");
    assert_eq!(schedule.account_id, "acc-1");
    assert_eq!(schedule.scheduled_at, "2026-12-31T10:00:00Z");
}

#[tokio::test]
async fn test_list_schedules_empty() {
    let store = setup_store().await;
    let schedules = store
        .list_schedules(&ScheduleFilter::default())
        .await
        .expect("list_schedules failed");
    assert!(schedules.is_empty());
}

#[tokio::test]
async fn test_cancel_schedule_existing() {
    let store = setup_store().await;
    create_test_account(&store, "acc-1", "user1").await;
    create_test_schedule(&store, "sched-1", "acc-1").await;

    store
        .delete_schedule("sched-1")
        .await
        .expect("cancel_schedule should succeed");

    let result = store.get_schedule("sched-1").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_cancel_schedule_nonexistent_returns_error() {
    let store = setup_store().await;
    let result = store.delete_schedule("nonexistent-id").await;
    assert!(
        result.is_err(),
        "Cancelling nonexistent schedule should error"
    );
}

#[tokio::test]
async fn test_get_schedule_nonexistent_returns_error() {
    let store = setup_store().await;
    let result = store.get_schedule("nonexistent-id").await;
    assert!(result.is_err());
}

// ═══════════════════════════════════════════════════════════════
// Comment operations (fetch_comments, get_post_sentiment)
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_list_comments_on_nonexistent_post() {
    let store = setup_store().await;
    let comments = store
        .list_comments("nonexistent-post", &CommentFilter::default())
        .await
        .expect("list_comments should not error for missing post");
    assert!(comments.is_empty());
}

#[tokio::test]
async fn test_insert_and_list_comment() {
    let store = setup_store().await;
    create_test_account(&store, "acc-1", "user1").await;
    create_test_post(&store, "post-1", "acc-1", "test post").await;

    let comment = store
        .insert_comment(
            "comment-1",
            "post-1",
            Some("commenter"),
            Some("user_456"),
            "Great post!",
        )
        .await
        .expect("insert_comment failed");

    assert_eq!(comment.text, "Great post!");
    assert_eq!(comment.author_username.as_deref(), Some("commenter"));

    let comments = store
        .list_comments("post-1", &CommentFilter::default())
        .await
        .expect("list_comments failed");
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].text, "Great post!");
}

// ═══════════════════════════════════════════════════════════════
// HITL operations (approve_schedule, reject_schedule)
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_approve_draft_schedule() {
    let store = setup_store().await;
    create_test_account(&store, "acc-1", "user1").await;

    let input = CreateSchedule {
        account_id: "acc-1".to_string(),
        caption: Some("Needs approval".to_string()),
        media_type: Some("TEXT".to_string()),
        text_attachment: None,
        media_urls: None,
        scheduled_at: "2026-12-31T10:00:00Z".to_string(),
        location_id: None,
        auto_approve: false,
    };
    let sched = store
        .create_schedule("sched-1", &input)
        .await
        .expect("create_schedule failed");
    assert_eq!(sched.status, "draft");

    let approved = store
        .approve_schedule("sched-1", Some("admin"))
        .await
        .expect("approve_schedule failed");
    assert_eq!(approved.status, "pending");
    assert_eq!(approved.approved_by.as_deref(), Some("admin"));
}

#[tokio::test]
async fn test_reject_draft_schedule() {
    let store = setup_store().await;
    create_test_account(&store, "acc-1", "user1").await;

    let input = CreateSchedule {
        account_id: "acc-1".to_string(),
        caption: Some("Will be rejected".to_string()),
        media_type: Some("TEXT".to_string()),
        text_attachment: None,
        media_urls: None,
        scheduled_at: "2026-12-31T10:00:00Z".to_string(),
        location_id: None,
        auto_approve: false,
    };
    store
        .create_schedule("sched-1", &input)
        .await
        .expect("create_schedule failed");

    let rejected = store
        .reject_schedule("sched-1", Some("spam content"))
        .await
        .expect("reject_schedule failed");
    assert_eq!(rejected.status, "rejected");
    assert_eq!(rejected.error.as_deref(), Some("spam content"));
}

// ═══════════════════════════════════════════════════════════════
// Media operations (list_media)
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_list_media_empty() {
    let store = setup_store().await;
    let media = store
        .list_media(&MediaFilter::default())
        .await
        .expect("list_media failed");
    assert!(media.is_empty());
}

// ═══════════════════════════════════════════════════════════════
// Mentions operations (list_mentions)
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_list_mentions_empty() {
    let store = setup_store().await;
    let mentions = store
        .list_mentions(&MentionFilter {
            account_id: Some("acc-1".to_string()),
            ..Default::default()
        })
        .await
        .expect("list_mentions failed");
    assert!(mentions.is_empty());
}

// ═══════════════════════════════════════════════════════════════
// Analytics trend operations (get_post_trend)
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_get_post_trend_no_snapshots() {
    let store = setup_store().await;
    let snaps = store
        .list_analytics_snap("nonexistent-post")
        .await
        .expect("list_analytics_snap should not error for missing post");
    assert!(snaps.is_empty());
}
