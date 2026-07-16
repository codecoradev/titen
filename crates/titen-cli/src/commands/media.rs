use anyhow::Result;
use clap::Subcommand;

use crate::api::{TitenApi, TitenConfig, print_data};

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
    /// Delete media
    Delete { id: String },
}

pub async fn run(action: MediaAction) -> Result<()> {
    let config = TitenConfig::from_env();
    let api = TitenApi::new(&config);

    match action {
        MediaAction::List => {
            let resp = api.get("/api/media").await?;
            print_data(&resp);
        }
        MediaAction::Upload {
            file_path,
            content_type,
        } => {
            // Read file bytes
            let data = tokio::fs::read(&file_path)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to read file {file_path}: {e}"))?;

            let ct = content_type.unwrap_or_else(|| "application/octet-stream".into());

            // Build multipart
            let url = format!("{}{}", config.base_url, "/api/media");
            let mut req = reqwest::Client::new().post(&url);

            if let Some(key) = &config.api_key {
                req = req.header("X-API-Key", key);
            }

            let part = reqwest::multipart::Part::bytes(data)
                .file_name(
                    std::path::Path::new(&file_path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("file")
                        .to_string(),
                )
                .mime_str(&ct)?;

            let form = reqwest::multipart::Form::new().part("file", part);
            req = req.multipart(form);

            let resp = req.send().await?;
            let json: serde_json::Value = resp.json().await?;
            print_data(&json);
        }
        MediaAction::Delete { id } => {
            let resp = api.delete(&format!("/api/media/{id}")).await?;
            print_data(&resp);
        }
    }
    Ok(())
}
