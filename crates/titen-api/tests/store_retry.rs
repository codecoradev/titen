//! #257 — transient-error retry mechanics at the store level.
//!
//! The scheduler's decision logic (which errors are transient, when to give
//! up) is covered by unit tests in `titen-core/src/error.rs`. These tests
//! verify the DB half: deferral backfills `next_due_at` (eligibility gate),
//! `get_due_schedules` honors that gate, and final failures persist the
//! attempt count.

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
    pool
}

async fn seed_pending(pool: &SqlitePool, id: &str) {
    // schedules.account_id has an FK to accounts — seed the parent first.
    sqlx::query(
        "INSERT INTO accounts (id, username, user_id, access_token, expires_at, is_active, created_at, updated_at) \
         VALUES ('acc-1', 'retrytest', '987654321', 'FAKE_TEST_TOKEN', '2026-12-01T00:00:00+00:00', 1, datetime('now'), datetime('now')) \
         ON CONFLICT(id) DO NOTHING",
    )
    .execute(pool)
    .await
    .expect("seed account");

    // Insert directly: the retry columns are scheduler-owned, and the API
    // route has no reason to set them.
    sqlx::query(
        "INSERT INTO schedules (id, account_id, media_type, caption, scheduled_at, status, attempt_count, created_at, updated_at) \
         VALUES (?, 'acc-1', 'CAROUSEL', 'retry test', '2026-09-09T10:00:00+07:00', 'pending', 0, datetime('now'), datetime('now'))",
    )
    .bind(id)
    .execute(pool)
    .await
    .expect("seed schedule");
}

#[tokio::test]
async fn defer_sets_future_gate_and_resets_pending() {
    let pool = pool().await;
    let store = titen_core::Store::new(pool.clone());
    seed_pending(&pool, "sched-defer-1").await;

    store
        .defer_failed_schedule("sched-defer-1", 1, 600, "transient boom")
        .await
        .expect("defer");

    let row: (String, i64, Option<String>) = sqlx::query_as(
        "SELECT status, attempt_count, next_due_at FROM schedules WHERE id = 'sched-defer-1'",
    )
    .fetch_one(&pool)
    .await
    .expect("row");

    assert_eq!(row.0, "pending");
    assert_eq!(row.1, 1);
    let gate = row.2.expect("next_due_at backfilled");
    // Gate must be in the future relative to now (10 min backoff).
    let still_due: Vec<titen_core::models::Schedule> =
        store.get_due_schedules().await.expect("due query");
    assert!(
        !still_due.iter().any(|s| s.id == "sched-defer-1"),
        "deferred schedule must NOT be due while the gate is in the future (gate={gate})"
    );
}

#[tokio::test]
async fn deferred_schedule_becomes_due_after_gate_passes() {
    let pool = pool().await;
    let store = titen_core::Store::new(pool.clone());
    seed_pending(&pool, "sched-defer-2").await;

    store
        .defer_failed_schedule("sched-defer-2", 1, 600, "transient boom")
        .await
        .expect("defer");

    // Simulate the 10 minutes elapsing.
    sqlx::query("UPDATE schedules SET next_due_at = datetime('now', '-1 second') WHERE id = 'sched-defer-2'")
        .execute(&pool)
        .await
        .expect("age gate");

    let due = store.get_due_schedules().await.expect("due query");
    assert!(due.iter().any(|s| s.id == "sched-defer-2"));
}

#[tokio::test]
async fn schedules_without_gate_still_use_scheduled_at() {
    // Regression guard: rows created before migration 018 (next_due_at NULL)
    // must keep the original eligibility semantics.
    let pool = pool().await;
    let store = titen_core::Store::new(pool.clone());
    seed_pending(&pool, "sched-legacy").await;

    let due = store.get_due_schedules().await.expect("due query");
    assert!(due.iter().any(|s| s.id == "sched-legacy"));
}

#[tokio::test]
async fn final_failure_persists_attempt_count() {
    let pool = pool().await;
    let store = titen_core::Store::new(pool.clone());
    seed_pending(&pool, "sched-final-1").await;

    store
        .fail_schedule_final("sched-final-1", 2, "gave up after retry")
        .await
        .expect("fail final");

    let row: (String, i64, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT status, attempt_count, next_due_at, error FROM schedules WHERE id = 'sched-final-1'",
    )
    .fetch_one(&pool)
    .await
    .expect("row");

    assert_eq!(row.0, "failed");
    assert_eq!(row.1, 2);
    assert_eq!(row.2, None, "final failure must not set a retry gate");
    assert_eq!(row.3.expect("error"), "gave up after retry");
}
