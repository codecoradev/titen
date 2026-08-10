use anyhow::Result;
use clap::Subcommand;

use crate::api::{TitenApi, TitenConfig, print_data};

#[derive(Subcommand, Debug)]
pub enum CommentAction {
    /// Fetch comments from Threads API
    Fetch { post_id: String },
    /// List stored comments
    List { post_id: String },
    /// Analyze sentiment
    Sentiment { post_id: String },
}

pub async fn run(action: CommentAction) -> Result<()> {
    let config = TitenConfig::from_env();
    let api = TitenApi::new(&config);

    match action {
        CommentAction::Fetch { post_id } => {
            let resp = api
                .post(
                    &format!("/api/posts/{post_id}/comments/fetch"),
                    serde_json::json!({}),
                )
                .await?;
            print_data(&resp);
        }
        CommentAction::List { post_id } => {
            let resp = api.get(&format!("/api/posts/{post_id}/comments")).await?;
            print_data(&resp);
        }
        CommentAction::Sentiment { post_id } => {
            let resp = api
                .get(&format!("/api/posts/{post_id}/comments/sentiment"))
                .await?;
            print_data(&resp);
        }
    }
    Ok(())
}
