//! OAuth state token tests (#237 — CSRF protection):
//! insertion, owner-bound single-use consumption, expiry rejection,
//! legacy consume-any semantics, and existence checks.

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use titen_core::Store;

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

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[tokio::test]
async fn insert_then_consume_bound_succeeds_once() {
    let store = Store::new(pool().await);
    store.migrate().await.expect("migrate");

    store
        .insert_oauth_state("tok-1", "key-a", now() + 600)
        .await
        .expect("insert");

    // First consumption by the owning key succeeds…
    assert!(
        store
            .consume_oauth_state("tok-1", "key-a")
            .await
            .expect("consume")
    );

    // …second consumption fails (single use).
    assert!(
        !store
            .consume_oauth_state("tok-1", "key-a")
            .await
            .expect("consume-2")
    );
}

#[tokio::test]
async fn consume_rejects_wrong_owner() {
    let store = Store::new(pool().await);
    store.migrate().await.expect("migrate");

    store
        .insert_oauth_state("tok-2", "key-a", now() + 600)
        .await
        .expect("insert");

    // A different session/key cannot consume someone else's state.
    assert!(
        !store
            .consume_oauth_state("tok-2", "key-b")
            .await
            .expect("consume")
    );

    // The token is still valid for its real owner afterwards.
    assert!(store.oauth_state_exists("tok-2").await.expect("exists"));
}

#[tokio::test]
async fn consume_rejects_expired_token() {
    let store = Store::new(pool().await);
    store.migrate().await.expect("migrate");

    store
        .insert_oauth_state("tok-3", "key-a", now() - 1)
        .await
        .expect("insert");

    assert!(
        !store
            .consume_oauth_state("tok-3", "key-a")
            .await
            .expect("consume")
    );
    assert!(!store.oauth_state_exists("tok-3").await.expect("exists"));
}

#[tokio::test]
async fn legacy_consume_any_is_single_use() {
    let store = Store::new(pool().await);
    store.migrate().await.expect("migrate");

    store
        .insert_oauth_state("tok-4", "key-a", now() + 600)
        .await
        .expect("insert");

    assert!(store.consume_any_oauth_state("tok-4").await.expect("any"));
    assert!(!store.consume_any_oauth_state("tok-4").await.expect("any2"));
    // And the owner can no longer use it either.
    assert!(
        !store
            .consume_oauth_state("tok-4", "key-a")
            .await
            .expect("consume")
    );
}

#[tokio::test]
async fn prune_removes_only_expired() {
    let store = Store::new(pool().await);
    store.migrate().await.expect("migrate");

    store
        .insert_oauth_state("tok-live", "key-a", now() + 600)
        .await
        .expect("insert");
    store
        .insert_oauth_state("tok-dead", "key-a", now() - 10)
        .await
        .expect("insert");

    store.prune_expired_oauth_states().await.expect("prune");

    assert!(store.oauth_state_exists("tok-live").await.expect("live"));
    assert!(!store.oauth_state_exists("tok-dead").await.expect("dead"));
}

#[tokio::test]
async fn unknown_token_is_rejected() {
    let store = Store::new(pool().await);
    store.migrate().await.expect("migrate");

    assert!(
        !store
            .consume_oauth_state("never-inserted", "key-a")
            .await
            .expect("consume")
    );
}
