use anyhow::Result;
use clap::Subcommand;
use serde_json::json;

use crate::api::{TitenApi, TitenConfig, print_data};

#[derive(Subcommand)]
pub enum PostAction {
    /// Create and publish a post
    Create {
        account: String,
        #[arg(short, long)]
        text: String,
        #[arg(long)]
        media_type: Option<String>,
        #[arg(long)]
        image_url: Option<String>,
    },
    /// List posts
    List {
        #[arg(short, long)]
        account: Option<String>,
        #[arg(long)]
        status: Option<String>,
    },
    /// Delete a post
    Delete { post_id: String },
    /// Fetch insights for a post
    Insights { post_id: String },
}

pub async fn run(action: PostAction) -> Result<()> {
    let config = TitenConfig::from_env();
    let api = TitenApi::new(&config);

    match action {
        PostAction::Create {
            account,
            text,
            media_type,
            image_url,
        } => {
            let body = json!({
                "account_id": account,
                "caption": text,
                "media_type": media_type.unwrap_or_else(|| "TEXT".into()),
                "image_url": image_url,
            });
            let resp = api.post("/api/posts", body).await?;
            print_data(&resp);
        }
        PostAction::List { account, status } => {
            let mut path = "/api/posts?".to_string();
            if let Some(a) = account {
                path.push_str(&format!("account_id={a}&"));
            }
            if let Some(s) = status {
                path.push_str(&format!("status={s}&"));
            }
            let resp = api.get(&path).await?;
            print_data(&resp);
        }
        PostAction::Delete { post_id } => {
            let resp = api.delete(&format!("/api/posts/{post_id}")).await?;
            print_data(&resp);
        }
        PostAction::Insights { post_id } => {
            let resp = api.get(&format!("/api/posts/{post_id}/insights")).await?;
            print_data(&resp);
        }
    }
    Ok(())
}
