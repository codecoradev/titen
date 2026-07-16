use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum AccountAction {
    /// List all accounts
    List,
    /// Add a new account
    Add {
        username: String,
        user_id: String,
        access_token: String,
        expires_at: String,
    },
    /// Remove an account
    Remove { id_or_username: String },
    /// Refresh account token
    Refresh { id_or_username: String },
    /// Show account token status
    Status { id_or_username: String },
}

pub async fn run(action: AccountAction) -> Result<()> {
    match action {
        AccountAction::List => {
            println!("Listing accounts... (not yet connected to DB)");
        }
        AccountAction::Add {
            username,
            user_id,
            access_token,
            expires_at,
        } => {
            println!("Adding account: {} (user_id: {})", username, user_id);
            println!("  Token: {}...", &access_token[..8.min(access_token.len())]);
            println!("  Expires: {}", expires_at);
        }
        AccountAction::Remove { id_or_username } => {
            println!("Removing account: {}", id_or_username);
        }
        AccountAction::Refresh { id_or_username } => {
            println!("Refreshing token for: {}", id_or_username);
        }
        AccountAction::Status { id_or_username } => {
            println!("Token status for: {}", id_or_username);
        }
    }
    Ok(())
}

pub async fn token_check() -> Result<()> {
    println!("Checking token expiry for all accounts... (not yet connected to DB)");
    Ok(())
}
