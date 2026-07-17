use anyhow::Result;
use clap::Subcommand;
use serde_json::json;

use crate::api::{TitenApi, TitenConfig, print_data};

#[derive(Subcommand)]
pub enum AccountAction {
    /// List all accounts
    List,
    /// Add a new account (username + user_id auto-resolved from token)
    Add {
        #[arg(long)]
        access_token: String,
        #[arg(long)]
        app_secret: Option<String>,
        #[arg(long)]
        username: Option<String>,
        #[arg(long)]
        user_id: Option<String>,
        #[arg(long)]
        expires_at: Option<String>,
    },
    /// Remove an account
    Remove { id: String },
    /// Refresh account token
    Refresh { id: String },
    /// Check token expiry
    TokenCheck { id: String },
}

pub async fn run(action: AccountAction) -> Result<()> {
    let config = TitenConfig::from_env();
    let api = TitenApi::new(&config);

    match action {
        AccountAction::List => {
            let resp = api.get("/api/accounts").await?;
            print_data(&resp);
        }
        AccountAction::Add {
            access_token,
            app_secret,
            username,
            user_id,
            expires_at,
        } => {
            let body = json!({
                "access_token": access_token,
                "app_secret": app_secret,
                "username": username,
                "user_id": user_id,
                "expires_at": expires_at.unwrap_or_else(|| {
                    (chrono::Utc::now() + chrono::Duration::days(60)).to_rfc3339()
                }),
            });
            let resp = api.post("/api/accounts", body).await?;
            print_data(&resp);
        }
        AccountAction::Remove { id } => {
            let resp = api.delete(&format!("/api/accounts/{id}")).await?;
            print_data(&resp);
        }
        AccountAction::Refresh { id } => {
            let resp = api
                .post(&format!("/api/accounts/{id}/refresh-token"), json!({}))
                .await?;
            print_data(&resp);
        }
        AccountAction::TokenCheck { id } => {
            let resp = api.get(&format!("/api/accounts/{id}")).await?;
            let data = resp.get("data").cloned().unwrap_or_default();
            let is_active = data
                .get("is_active")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let token_status = data
                .get("token_status")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let expires = data
                .get("expires_at")
                .and_then(|v| v.as_str())
                .unwrap_or("never");
            println!("Account: {id}");
            println!("  Active: {is_active}");
            println!("  Token: {token_status}");
            println!("  Expires: {expires}");
        }
    }
    Ok(())
}

pub async fn token_check() -> Result<()> {
    let config = TitenConfig::from_env();
    let api = TitenApi::new(&config);

    let resp = api.get("/api/accounts").await?;
    let accounts = resp
        .get("data")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();

    if accounts.is_empty() {
        println!("No accounts registered.");
        return Ok(());
    }

    println!("Token Status ({len} accounts):", len = accounts.len());
    for acct in &accounts {
        let id = acct.get("id").and_then(|v| v.as_str()).unwrap_or("?");
        let username = acct.get("username").and_then(|v| v.as_str()).unwrap_or("?");
        let status = acct
            .get("token_status")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let expires = acct
            .get("expires_at")
            .and_then(|v| v.as_str())
            .unwrap_or("never");
        let icon = match status {
            "valid" => "✓",
            "expiring" => "⚠",
            "expired" => "✗",
            _ => "?",
        };
        println!("  {icon} {username} ({id}) — {status}, expires: {expires}");
    }
    Ok(())
}
