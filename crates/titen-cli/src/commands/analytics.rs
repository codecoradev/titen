use anyhow::Result;
use clap::Subcommand;

use crate::api::{TitenApi, TitenConfig, print_data};

#[derive(Subcommand)]
pub enum AnalyticsAction {
    /// Show account post analytics
    Posts {
        account: String,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        to: Option<String>,
    },
    /// Show trend for a specific post
    Trend { post_id: String },
}

pub async fn run(action: AnalyticsAction) -> Result<()> {
    let config = TitenConfig::from_env();
    let api = TitenApi::new(&config);

    match action {
        AnalyticsAction::Posts { account, from, to } => {
            let mut path = format!("/api/analytics/posts?account_id={account}");
            if let Some(f) = from {
                path.push_str(&format!("&from={f}"));
            }
            if let Some(t) = to {
                path.push_str(&format!("&to={t}"));
            }
            let resp = api.get(&path).await?;
            print_data(&resp);
        }
        AnalyticsAction::Trend { post_id } => {
            let resp = api
                .get(&format!("/api/analytics/posts/{post_id}/trend"))
                .await?;
            print_data(&resp);
        }
    }
    Ok(())
}
