//! Titen CLI — library facade for testing.
//!
//! Re-exports the CLI argument structures and command modules so that
//! integration tests in `tests/` can parse and inspect clap commands.

pub mod api;
pub mod commands;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "titen", about = "Self-hosted Threads management platform")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start HTTP server
    Serve {
        #[arg(short, long, default_value = "7845")]
        port: u16,
        #[arg(short = 'H', long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long)]
        mcp: bool,
    },
    /// Manage Threads accounts
    Account {
        #[command(subcommand)]
        action: commands::account::AccountAction,
    },
    /// Create and manage posts
    Post {
        #[command(subcommand)]
        action: commands::post::PostAction,
    },
    /// Manage scheduled posts
    Schedule {
        #[command(subcommand)]
        action: commands::schedule::ScheduleAction,
    },
    /// Fetch and analyze comments
    Comment {
        #[command(subcommand)]
        action: commands::comment::CommentAction,
    },
    /// View analytics
    Analytics {
        #[command(subcommand)]
        action: commands::analytics::AnalyticsAction,
    },
    /// Manage media assets
    Media {
        #[command(subcommand)]
        action: commands::media::MediaAction,
    },
    /// Check all account token expiry status
    TokenCheck,
    /// Backup the SQLite database to a file
    Backup {
        /// Output file path (default: titen-backup-YYYYMMDD-HHMMSS.db)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Restore the SQLite database from a backup file
    Restore {
        /// Backup file to restore from
        #[arg(short, long)]
        input: String,
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },
}
