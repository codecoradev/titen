use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum PostAction {
    /// Create and publish a post
    Create {
        account: String,
        #[arg(short, long)]
        text: String,
        #[arg(short = 't', long, default_value = "TEXT")]
        media_type: String,
        #[arg(long)]
        image_url: Option<String>,
        #[arg(long)]
        attachment: Option<String>,
    },
    /// Delete a post
    Delete { post_id: String },
    /// Fetch post insights
    Insights { post_id: String },
}

pub async fn run(action: PostAction) -> Result<()> {
    match action {
        PostAction::Create {
            account,
            text,
            media_type,
            image_url,
            attachment,
        } => {
            println!("Creating post on account: {}", account);
            println!("  Type: {}", media_type);
            println!("  Text: {}", text);
            if let Some(url) = image_url {
                println!("  Image: {}", url);
            }
            if let Some(att) = attachment {
                println!("  Attachment: {}", &att[..50.min(att.len())]);
            }
        }
        PostAction::Delete { post_id } => {
            println!("Deleting post: {}", post_id);
        }
        PostAction::Insights { post_id } => {
            println!("Fetching insights for post: {}", post_id);
        }
    }
    Ok(())
}
