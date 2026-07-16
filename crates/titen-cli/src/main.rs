use clap::{Parser, Subcommand};

use anyhow::Result;

mod api;
mod commands;

#[derive(Parser)]
#[command(name = "titen", about = "Self-hosted Threads management platform")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start HTTP server
    Serve {
        #[arg(short, long, default_value = "7845")]
        port: u16,
        #[arg(short, long, default_value = "0.0.0.0")]
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
}

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
    }
}
