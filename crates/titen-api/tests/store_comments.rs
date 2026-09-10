//! #261 — comment persistence: threads_comment_id stored, dedup by id,
//! text-fallback dedup for id-less rows (Meta standard access omits id/from),
//! and in-place attribution backfill.

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::str::FromStr;

async fn pool() -> SqlitePool {
    let options = SqliteConnectOptions::from_str("sqlite::memory:").expect("in-memory sqlite");
    let pool = SqlitePoolOptions::new()
        .max_connections(1) // single connection = single in-memory DB
        .connect_with(options)
        .await
        .expect("pool");
    let store = titen_core::Store::new(pool.clone());
    store.migrate().await.expect("migrations");
    seed_post(&store, &pool).await;
    pool
}

async fn seed_post(store: &titen_core::Store, pool: &SqlitePool) {
    sqlx::query(
        "INSERT INTO accounts (id, username, user_id, access_token, expires_at, is_active, created_at, updated_at) \
         VALUES ('acc-1', 'cmttest', '123654', 'FAKE_TEST_TOKEN', '2026-12-01T00:00:00+00:00', 1, datetime('now'), datetime('now')) \
         ON CONFLICT(id) DO NOTHING",
    )
    .execute(pool)
    .await
    .expect("seed account");

    let input = titen_core::models::CreatePost {
        account_id: "acc-1".into(),
        media_type: Some("CAROUSEL".into()),
        caption: Some("post under test".into()),
        text_attachment: None,
        image_url: None,
        video_url: None,
        image_urls: None,
        media_ids: None,
        alt_text: None,
        reply_to_id: None,
    };
    store
        .create_post("post-1", &input)
        .await
        .expect("seed post");
}

#[tokio::test]
async fn threads_comment_id_is_persisted() {
    let pool = pool().await;
    let store = titen_core::Store::new(pool.clone());

    let c = store
        .insert_comment(
            "c1",
            "post-1",
            Some("17891234567890123"),
            Some("alice"),
            Some("u1"),
            "hello",
        )
        .await
        .expect("insert");

    assert_eq!(c.threads_comment_id.as_deref(), Some("17891234567890123"));
    assert_eq!(c.author_username.as_deref(), Some("alice"));
}

#[tokio::test]
async fn same_threads_id_dedups_to_one_row() {
    let pool = pool().await;
    let store = titen_core::Store::new(pool.clone());

    let a = store
        .insert_comment("c1", "post-1", Some("tc-1"), Some("alice"), None, "hello")
        .await
        .expect("first");
    let b = store
        .insert_comment("c2", "post-1", Some("tc-1"), Some("alice"), None, "hello")
        .await
        .expect("second (dup)");

    assert_eq!(
        a.id, b.id,
        "re-fetch of the same comment must reuse the row"
    );
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM comments WHERE post_id = 'post-1'")
        .fetch_one(&pool)
        .await
        .expect("count");
    assert_eq!(count.0, 1);
}

#[tokio::test]
async fn idless_comments_dedup_by_text() {
    let pool = pool().await;
    let store = titen_core::Store::new(pool.clone());

    let a = store
        .insert_comment("c1", "post-1", None, None, None, "anonymous question")
        .await
        .expect("first");
    let b = store
        .insert_comment("c2", "post-1", None, None, None, "anonymous question")
        .await
        .expect("second (dup)");

    assert_eq!(a.id, b.id, "id-less re-fetch must dedup by (post_id, text)");
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM comments WHERE post_id = 'post-1'")
        .fetch_one(&pool)
        .await
        .expect("count");
    assert_eq!(count.0, 1);
}

#[tokio::test]
async fn attribution_backfill_fills_legacy_row_in_place() {
    let pool = pool().await;
    let store = titen_core::Store::new(pool.clone());

    // Legacy fetch: Meta returned no id/from (standard access).
    let legacy = store
        .insert_comment("c1", "post-1", None, None, None, "How does supersede work?")
        .await
        .expect("legacy row");
    assert_eq!(legacy.threads_comment_id, None);

    // Later fetch (or enrichment): id + author now known, same text.
    let backfilled = store
        .insert_comment(
            "c2",
            "post-1",
            Some("tc-backfill"),
            Some("taufikabayy"),
            Some("user_9"),
            "How does supersede work?",
        )
        .await
        .expect("backfilled");

    assert_eq!(
        backfilled.id, legacy.id,
        "backfill must update in place, not insert"
    );
    assert_eq!(
        backfilled.threads_comment_id.as_deref(),
        Some("tc-backfill")
    );
    assert_eq!(backfilled.author_username.as_deref(), Some("taufikabayy"));
    assert_eq!(backfilled.author_user_id.as_deref(), Some("user_9"));

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM comments WHERE post_id = 'post-1'")
        .fetch_one(&pool)
        .await
        .expect("count");
    assert_eq!(count.0, 1);
}

#[tokio::test]
async fn different_idless_comments_are_not_collapsed() {
    let pool = pool().await;
    let store = titen_core::Store::new(pool.clone());

    store
        .insert_comment("c1", "post-1", None, None, None, "first question")
        .await
        .expect("first");
    store
        .insert_comment("c2", "post-1", None, None, None, "second question")
        .await
        .expect("second");

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM comments WHERE post_id = 'post-1'")
        .fetch_one(&pool)
        .await
        .expect("count");
    assert_eq!(count.0, 2, "distinct texts must stay distinct rows");
}

#[tokio::test]
async fn same_text_from_different_authors_stays_distinct() {
    let pool = pool().await;
    let store = titen_core::Store::new(pool.clone());

    // Meta omits id AND author for both — but authors were known on earlier
    // fetches. Identical text from different users must NOT collapse.
    store
        .insert_comment("c1", "post-1", None, Some("alice"), None, "Great post!")
        .await
        .expect("alice row");
    store
        .insert_comment("c2", "post-1", None, Some("bob"), None, "Great post!")
        .await
        .expect("bob row");

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM comments WHERE post_id = 'post-1'")
        .fetch_one(&pool)
        .await
        .expect("count");
    assert_eq!(count.0, 2, "same text + different authors = distinct rows");
}

#[tokio::test]
async fn backfill_never_grafts_onto_another_authors_row() {
    let pool = pool().await;
    let store = titen_core::Store::new(pool.clone());

    // alice's id-less comment…
    let alice = store
        .insert_comment(
            "c1",
            "post-1",
            None,
            Some("alice"),
            None,
            "How does supersede work?",
        )
        .await
        .expect("alice row");

    // …then the SAME text arrives attributed to taufikabayy (a different
    // person who coincidentally wrote the same words). It must become its
    // own row, NOT graft the id onto alice's row.
    let other = store
        .insert_comment(
            "c2",
            "post-1",
            Some("tc-x"),
            Some("taufikabayy"),
            Some("user_9"),
            "How does supersede work?",
        )
        .await
        .expect("taufik row");

    assert_ne!(alice.id, other.id);
    assert_eq!(
        alice.threads_comment_id, None,
        "alice's row must stay untouched"
    );
    assert_eq!(other.threads_comment_id.as_deref(), Some("tc-x"));

    let alice_after = store.get_comment(&alice.id).await.expect("reload");
    assert_eq!(alice_after.author_username.as_deref(), Some("alice"));
    assert_eq!(alice_after.threads_comment_id, None);
}
