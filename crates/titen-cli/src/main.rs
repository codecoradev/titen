//! Titen CLI — command-line interface for the Titen Threads management platform.
//!
//! Provides subcommands for all CRUD operations against the running HTTP API:
//! `serve`, `account`, `post`, `schedule`, `comment`, `analytics`, `media`, and `token-check`.
//!
//! The CLI talks to the API server over HTTP. Configure the server address with
//! `TITEN_URL` (default: `http://localhost:7845`) and authenticate with `TITEN_API_KEY`.

use anyhow::Result;
use clap::Parser;

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
    }
}
