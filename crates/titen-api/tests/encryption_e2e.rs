//! Integration test: verify tokens are encrypted at rest in SQLite.
//!
//! Sets TITEN_ENCRYPTION_KEY, creates a Store, inserts an account via
//! Store::create_account(), then reads the raw DB row to confirm the
//! stored values are ciphertext (enc:v1:...) not plaintext.

use sqlx::sqlite::SqlitePoolOptions;
use titen_core::{Store, models::CreateAccount};

#[tokio::test]
async fn tokens_are_encrypted_at_rest() {
    // Generate a valid 256-bit hex key (64 chars).
    let key: String = (0..32u8).map(|b| format!("{b:02x}")).collect();
    // SAFETY: This is a single-threaded test. No other code reads this env var concurrently.
    unsafe {
        std::env::set_var("TITEN_ENCRYPTION_KEY", &key);
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();

    let store = Store::new(pool.clone());
    store.migrate().await.unwrap();

    // Insert via Store::create_account (skips Threads API resolve).
    let id = uuid::Uuid::now_v7().to_string();
    let input = CreateAccount {
        username: Some("encryption_test_user".into()),
        user_id: Some("threads_123".into()),
        access_token: "SECRET_TOKEN_EAAB123".into(),
        expires_at: "2026-12-31T00:00:00Z".into(),
        app_id: Some("app_999".into()),
        app_secret: Some("SUPER_SECRET_APP_SECRET".into()),
    };
    store.create_account(&id, &input).await.unwrap();

    // Read through Store API (should get plaintext back).
    let account = store
        .get_account_by_username("encryption_test_user")
        .await
        .unwrap();
    assert_eq!(account.access_token, "SECRET_TOKEN_EAAB123");
    assert_eq!(
        account.app_secret.as_deref(),
        Some("SUPER_SECRET_APP_SECRET")
    );

    // Read raw DB row (should be encrypted ciphertext, NOT plaintext).
    let row: (String, Option<String>) =
        sqlx::query_as("SELECT access_token, app_secret FROM accounts WHERE username = ?")
            .bind("encryption_test_user")
            .fetch_one(&pool)
            .await
            .unwrap();

    assert!(
        row.0.starts_with("enc:v1:"),
        "access_token not encrypted in DB! Got starts with: {:?}",
        &row.0[..10]
    );
    assert!(
        row.1.as_ref().unwrap().starts_with("enc:v1:"),
        "app_secret not encrypted in DB!"
    );
    assert!(
        !row.0.contains("SECRET_TOKEN"),
        "Plaintext token leaked into DB!"
    );
    assert!(
        !row.1.as_ref().unwrap().contains("SUPER_SECRET"),
        "Plaintext secret leaked into DB!"
    );

    // SAFETY: End of single-threaded test.
    unsafe {
        std::env::remove_var("TITEN_ENCRYPTION_KEY");
    }
}
