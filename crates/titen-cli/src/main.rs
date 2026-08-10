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
