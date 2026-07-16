use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum MediaAction {
    /// List uploaded media
    List,
    /// Upload a file
    Upload {
        file_path: String,
        #[arg(long)]
        content_type: Option<String>,
    },
    /// Delete a media asset
    Delete { id: String },
}

pub async fn run(action: MediaAction) -> Result<()> {
    match action {
        MediaAction::List => {
            println!("Listing media assets...");
        }
        MediaAction::Upload { file_path, content_type } => {
            println!("Uploading: {}", file_path);
            if let Some(ct) = content_type { println!("  Content-Type: {}", ct); }
        }
        MediaAction::Delete { id } => {
            println!("Deleting media: {}", id);
        }
    }
    Ok(())
}
