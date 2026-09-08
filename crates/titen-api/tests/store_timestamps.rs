//! Timestamp canonicalization tests: mention/comment writes must store
//! canonical RFC3339 UTC, the migration must convert legacy rows, and the
//! trends-adjacent `list_mentions` date filters must see all variants.

use sqlx::{
    Row,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use titen_core::models::{CreateAccount, Mention, MentionFilter};
use titen_core::{Store, time};

async fn pool() -> sqlx::SqlitePool {
    let options = SqliteConnectOptions::new()
        .filename(":memory:")
        .create_if_missing(true);
    SqlitePoolOptions::new()
        .max_connections(1) // single connection = single in-memory DB
        .connect_with(options)
        .await
        .expect("connect")
}

async fn seed_account(store: &Store) -> String {
    let (account, _) = store
        .upsert_account(&CreateAccount {
            username: Some("codecoradev".into()),
            user_id: Some("17841400000000001".into()),
            access_token: "test-token".into(),
            expires_at: "2027-01-01T00:00:00Z".into(),
            app_id: None,
            app_secret: None,
        })
        .await
        .expect("seed account");
    account.id
}

fn mention(account_id: &str, external_id: &str, mentioned_at: Option<&str>) -> Mention {
    Mention {
        id: uuid::Uuid::now_v7().to_string(),
        account_id: account_id.to_string(),
        threads_mention_id: Some(external_id.to_string()),
        author_username: Some("tester".into()),
        author_user_id: None,
        text: Some("titen rocks".into()),
        media_type: None,
        permalink: None,
        mentioned_at: mentioned_at.map(String::from),
        fetched_at: chrono::Utc::now().to_rfc3339(),
    }
}

#[tokio::test]
async fn upsert_mention_canonicalizes_threads_offset() {
    let pool = pool().await;
    let store = Store::new(pool.clone());
    store.migrate().await.expect("migrate");
    let account_id = seed_account(&store).await;

    store
        .upsert_mention(&mention(
            &account_id,
            "m1",
            Some("2024-07-02T10:30:00+0000"),
        ))
        .await
        .expect("upsert");

    let row = sqlx::query("SELECT mentioned_at FROM mentions WHERE threads_mention_id = 'm1'")
        .fetch_one(&pool)
        .await
        .expect("row");
    let stored: String = row.get("mentioned_at");
    assert_eq!(stored, "2024-07-02T10:30:00Z");
}

#[tokio::test]
async fn upsert_mention_unparseable_timestamp_becomes_null() {
    let pool = pool().await;
    let store = Store::new(pool.clone());
    store.migrate().await.expect("migrate");
    let account_id = seed_account(&store).await;

    store
        .upsert_mention(&mention(&account_id, "m2", Some("yesterday probably")))
        .await
        .expect("upsert");

    let row = sqlx::query("SELECT mentioned_at FROM mentions WHERE threads_mention_id = 'm2'")
        .fetch_one(&pool)
        .await
        .expect("row");
    let stored: Option<String> = row.get("mentioned_at");
    assert_eq!(stored, None);
}

#[tokio::test]
async fn mention_fetched_at_is_canonical_and_conflict_updates_it() {
    let pool = pool().await;
    let store = Store::new(pool.clone());
    store.migrate().await.expect("migrate");
    let account_id = seed_account(&store).await;

    let m = mention(&account_id, "m3", None);
    store.upsert_mention(&m).await.expect("insert");
    // Re-auth of the same mention hits the ON CONFLICT branch.
    store.upsert_mention(&m).await.expect("conflict update");

    let row = sqlx::query("SELECT fetched_at FROM mentions WHERE threads_mention_id = 'm3'")
        .fetch_one(&pool)
        .await
        .expect("row");
    let stored: String = row.get("fetched_at");
    assert!(
        stored.ends_with('Z') && stored.len() == 20,
        "expected canonical RFC3339, got: {stored}"
    );
}

#[tokio::test]
async fn migration_converts_legacy_space_format_rows() {
    let pool = pool().await;
    let store = Store::new(pool.clone());
    store.migrate().await.expect("migrate");
    let account_id = seed_account(&store).await;

    // Simulate rows written by pre-canonicalization writers (schema default
    // `datetime('now')` and a raw Threads `+0000` timestamp).
    sqlx::query(
        "INSERT INTO mentions (id, account_id, threads_mention_id, text, mentioned_at, fetched_at)
         VALUES ('legacy-1', ?, 'legacy-1', 'old mention', '2024-07-02T10:30:00+0000', '2024-07-02 10:30:00')",
    )
    .bind(&account_id)
    .execute(&pool)
    .await
    .expect("insert legacy");

    // Re-run migrations — 013 is idempotent and must canonicalize the row.
    store.migrate().await.expect("remigrate");

    let row = sqlx::query(
        "SELECT mentioned_at, fetched_at FROM mentions WHERE threads_mention_id = 'legacy-1'",
    )
    .fetch_one(&pool)
    .await
    .expect("row");
    let mentioned_at: String = row.get("mentioned_at");
    let fetched_at: String = row.get("fetched_at");
    assert_eq!(mentioned_at, "2024-07-02T10:30:00Z");
    assert_eq!(fetched_at, "2024-07-02T10:30:00Z");
}

#[tokio::test]
async fn list_mentions_date_from_sees_all_format_variants() {
    let pool = pool().await;
    let store = Store::new(pool.clone());
    store.migrate().await.expect("migrate");
    let account_id = seed_account(&store).await;

    // Canonical write path…
    store
        .upsert_mention(&mention(
            &account_id,
            "m4",
            Some("2024-07-02T10:30:00+0000"),
        ))
        .await
        .expect("upsert");
    // …and a legacy space-format row (e.g. written before this fix).
    sqlx::query(
        "INSERT INTO mentions (id, account_id, threads_mention_id, text, fetched_at)
         VALUES ('legacy-2', ?, 'legacy-2', 'old mention', '2024-07-01 00:00:00')",
    )
    .bind(&account_id)
    .execute(&pool)
    .await
    .expect("insert legacy");
    store.migrate().await.expect("remigrate legacy row");

    let rows = store
        .list_mentions(&MentionFilter {
            account_id: Some(account_id.clone()),
            date_from: Some("2024-06-01T00:00:00Z".into()),
            ..Default::default()
        })
        .await
        .expect("list");
    let ids: Vec<String> = rows
        .iter()
        .filter_map(|m| m.threads_mention_id.clone())
        .collect();
    assert!(ids.contains(&"m4".to_string()), "canonical row missing");
    assert!(
        ids.contains(&"legacy-2".to_string()),
        "legacy row filtered out"
    );
}

#[tokio::test]
async fn comment_insert_fetched_at_is_canonical() {
    let pool = pool().await;
    let store = Store::new(pool.clone());
    store.migrate().await.expect("migrate");
    let account_id = seed_account(&store).await;

    let post = store
        .create_post(
            "post-t1",
            &titen_core::models::CreatePost {
                account_id: account_id.clone(),
                media_type: Some("TEXT".into()),
                caption: Some("hello".into()),
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
        .expect("create post");

    store
        .insert_comment("cmt-t1", &post.id, Some("tester"), None, "nice")
        .await
        .expect("insert comment");

    let row = sqlx::query("SELECT fetched_at FROM comments WHERE id = 'cmt-t1'")
        .fetch_one(&pool)
        .await
        .expect("row");
    let stored: String = row.get("fetched_at");
    assert!(
        stored.ends_with('Z') && stored.len() == 20,
        "expected canonical RFC3339, got: {stored}"
    );
}

#[test]
fn time_helper_accepts_titen_formats() {
    // Spot-check the shared helper the write/read paths rely on.
    assert_eq!(
        time::to_rfc3339_utc("2026-08-27 12:34:56").as_deref(),
        Some("2026-08-27T12:34:56Z")
    );
    assert_eq!(
        time::to_rfc3339_utc("2026-08-27T12:34:56+07:00").as_deref(),
        Some("2026-08-27T05:34:56Z")
    );
}
