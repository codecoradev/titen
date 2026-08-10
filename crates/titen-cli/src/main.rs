//! Titen CLI — command-line interface for the Titen Threads management platform.
//!
//! Provides subcommands for all CRUD operations against the running HTTP API:
//! `serve`, `account`, `post`, `schedule`, `comment`, `analytics`, `media`, and `token-check`.
//!
//! The CLI talks to the API server over HTTP. Configure the server address with
//! `TITEN_URL` (default: `http://localhost:7845`) and authenticate with `TITEN_API_KEY`.

use anyhow::Result;
use clap::Parser;
use std::time::SystemTime;

use titen_cli::{Cli, Commands, commands};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { port, host, mcp } => commands::serve::run(&host, port, mcp).await,
        Commands::Account { action } => commands::account::run(action).await,
        Commands::Post { action } => commands::post::run(action).await,
        Commands::Schedule { action } => commands::schedule::run(action).await,
        Commands::Comment { action } => commands::comment::run(action).await,
        Commands::Analytics { action } => commands::analytics::run(action).await,
        Commands::Media { action } => commands::media::run(action).await,
        Commands::TokenCheck => commands::account::token_check().await,
        Commands::Status => show_status().await,
        Commands::Backup { output } => backup_database(output).await,
        Commands::Restore { input, yes } => restore_database(&input, yes).await,
    }
}

/// Get the database path from TITEN_DB_PATH env or default.
fn db_path() -> String {
    std::env::var("TITEN_DB_PATH").unwrap_or_else(|_| "titen.db".to_string())
}

/// Backup the SQLite database using `VACUUM INTO` for a consistent snapshot.
async fn backup_database(output: Option<String>) -> Result<()> {
    let source = db_path();

    if !std::path::Path::new(&source).exists() {
        anyhow::bail!("Database file not found: {source}");
    }

    let dest = match output {
        Some(p) => p,
        None => {
            let timestamp = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_secs();
            let datetime = chrono::DateTime::from_timestamp(timestamp as i64, 0)
                .unwrap_or_default()
                .format("%Y%m%d-%H%M%S");
            format!("titen-backup-{datetime}.db")
        }
    };

    eprintln!("Backing up {source} → {dest}");

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&format!("sqlite://{source}"))
        .await?;
    // Validate path to prevent SQL injection via VACUUM INTO.
    // SQLite VACUUM INTO doesn't support bound parameters, so we use a strict
    // allowlist of safe path characters before interpolation.
    // CRITICAL: Single quote (') must NEVER be added to this allowlist —
    // it would reintroduce SQL injection. The allowlist approach deliberately
    // excludes it rather than relying on a denylist.
    if !dest
        .chars()
        .all(|c| c.is_alphanumeric() || matches!(c, '/' | '\\' | '-' | '_' | '.' | ' ' | ':'))
    {
        anyhow::bail!(
            "Invalid backup path: '{dest}' contains unsafe characters. \
             Only alphanumeric, path separators (/ \\), '-', '_', '.', ':', and spaces are allowed."
        );
    }
    sqlx::query(&format!("VACUUM INTO '{dest}'"))
        .execute(&pool)
        .await?;
    pool.close().await;

    let size = std::fs::metadata(&dest)?.len();
    eprintln!("✅ Backup complete: {dest} ({size} bytes)");
    Ok(())
}

/// Restore the SQLite database from a backup file.
async fn restore_database(input: &str, yes: bool) -> Result<()> {
    if !std::path::Path::new(input).exists() {
        anyhow::bail!("Backup file not found: {input}");
    }

    let dest = db_path();

    if !yes {
        eprintln!("⚠️  WARNING: Ensure the titen server is stopped before restoring.");
        eprintln!("   Restoring while the server is running may cause data corruption.");
        eprintln!();
        eprintln!("   This will REPLACE the current database: {dest}");
        eprintln!("   Source: {input}");
        eprint!("   Continue? [y/N] ");
        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf)?;
        if !buf.trim().eq_ignore_ascii_case("y") {
            eprintln!("Cancelled.");
            return Ok(());
        }
    }

    // Backup current DB before overwriting (safety net)
    if std::path::Path::new(&dest).exists() {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs();
        let datetime = chrono::DateTime::from_timestamp(timestamp as i64, 0)
            .unwrap_or_default()
            .format("%Y%m%d-%H%M%S");
        let backup_name = format!("{dest}.pre-restore-{datetime}");
        eprintln!("Saving current database to {backup_name}");
        std::fs::copy(&dest, &backup_name)?;
    }

    // Stop any active connections by removing the WAL/SHM files
    let wal_path = format!("{dest}-wal");
    let shm_path = format!("{dest}-shm");
    let _ = std::fs::remove_file(&wal_path);
    let _ = std::fs::remove_file(&shm_path);

    eprintln!("Restoring {input} → {dest}");
    std::fs::copy(input, &dest)?;

    let size = std::fs::metadata(&dest)?.len();
    eprintln!("✅ Restore complete: {dest} ({size} bytes)");
    Ok(())
}

/// Show system status: accounts, token health, post/schedule counts, DB info.
async fn show_status() -> Result<()> {
    let path = db_path();

    if !std::path::Path::new(&path).exists() {
        anyhow::bail!("Database not found: {path}. Start the server first with `titen serve`.");
    }

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&format!("sqlite:{path}"))
        .await?;

    // ── Version ──
    let version = env!("CARGO_PKG_VERSION");

    // ── Accounts ──
    let accounts: Vec<(String, String, String, bool)> = sqlx::query_as(
        "SELECT username, user_id, expires_at, is_active FROM accounts ORDER BY username",
    )
    .fetch_all(&pool)
    .await?;

    let total_accounts = accounts.len();
    let active_accounts = accounts.iter().filter(|(.., active)| *active).count();

    // Token expiry analysis
    let now = chrono::Utc::now();
    let mut expired = 0;
    let mut expiring_soon = 0;

    for (_, _, expires_at, _) in &accounts {
        match chrono::DateTime::parse_from_rfc3339(expires_at) {
            Ok(dt) => {
                let remaining = dt.with_timezone(&chrono::Utc) - now;
                if remaining.num_seconds() <= 0 {
                    expired += 1;
                } else if remaining.num_hours() < 24 {
                    expiring_soon += 1;
                }
            }
            Err(_) => expired += 1, // unparseable = treat as expired
        }
    }

    // ── Posts ──
    let total_posts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts")
        .fetch_one(&pool)
        .await?;

    let published_posts: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE threads_post_id IS NOT NULL")
            .fetch_one(&pool)
            .await?;

    // ── Schedules ──
    let pending_schedules: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM schedules WHERE status = 'pending'")
            .fetch_one(&pool)
            .await?;

    let approved_schedules: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM schedules WHERE status = 'approved'")
            .fetch_one(&pool)
            .await?;

    // ── Comments ──
    let total_comments: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM comments")
        .fetch_one(&pool)
        .await?;

    // ── Media ──
    let total_media: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM media")
        .fetch_one(&pool)
        .await?;

    // ── DB size ──
    let db_size = std::fs::metadata(&path)?.len();

    pool.close().await;

    // ── Print ──
    println!("┌─────────────────────────────────────────────┐");
    println!("│              TITEN STATUS                   │");
    println!("├─────────────────────────────────────────────┤");
    println!("│ Version:    {version:<33}│");
    println!("│ Database:   {path:<33}│");
    println!(
        "│ DB Size:    {:>8} bytes ({:>7})      │",
        db_size,
        format_size(db_size)
    );
    println!("├─────────────────────────────────────────────┤");
    println!("│ ACCOUNTS                                    │");
    println!("│   Total:     {total_accounts:<30} │");
    println!("│   Active:    {active_accounts:<30} │");
    println!("│   Expired:   {expired:<30} │");
    println!("│   Expiring:  {expiring_soon:<30} │");
    println!("├─────────────────────────────────────────────┤");
    println!("│ CONTENT                                     │");
    println!("│   Posts:     {total_posts:<4} ({published_posts} published)        │");
    println!("│   Schedules: {pending_schedules:<4} pending, {approved_schedules} approved   │");
    println!("│   Comments:  {total_comments:<30} │");
    println!("│   Media:     {total_media:<30} │");
    println!("├─────────────────────────────────────────────┤");

    // Token status per account
    if total_accounts > 0 {
        println!("│ TOKEN DETAILS                               │");
        for (username, _, expires_at, is_active) in &accounts {
            let status = match chrono::DateTime::parse_from_rfc3339(expires_at) {
                Ok(dt) => {
                    let remaining = dt.with_timezone(&chrono::Utc) - now;
                    if remaining.num_seconds() <= 0 {
                        "EXPIRED".to_string()
                    } else if remaining.num_hours() < 24 {
                        format!("{}h left", remaining.num_hours())
                    } else {
                        format!("{}d left", remaining.num_days())
                    }
                }
                Err(_) => "INVALID".to_string(),
            };
            let active_tag = if *is_active { "ON" } else { "OFF" };
            let line = format!("  @{username:<14} [{active_tag}]  {status}");
            println!("│ {line:<44}│",);
        }
        println!("├─────────────────────────────────────────────┤");
    }

    // Health summary
    let health = if expired > 0 {
        "DEGRADED".to_string()
    } else if expiring_soon > 0 {
        "WARNING".to_string()
    } else {
        "HEALTHY".to_string()
    };
    println!("│ Overall:    {health:<31} │");
    println!("└─────────────────────────────────────────────┘");

    Ok(())
}

/// Format bytes into human-readable string.
fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
