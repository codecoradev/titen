use anyhow::Result;
use clap::Subcommand;
use serde_json::json;

use crate::api::{TitenApi, TitenConfig, print_data};

#[derive(Subcommand)]
pub enum ScheduleAction {
    /// List scheduled posts
    List {
        #[arg(short, long)]
        account: Option<String>,
        #[arg(short, long)]
        status: Option<String>,
    },
    /// Create a new schedule
    Create {
        account: String,
        #[arg(short, long)]
        text: String,
        #[arg(short, long)]
        at: String,
        #[arg(long)]
        media_type: Option<String>,
    },
    /// Cancel a schedule
    Cancel { id: String },
    /// Show upcoming schedules
    Upcoming,
}

pub async fn run(action: ScheduleAction) -> Result<()> {
    let config = TitenConfig::from_env();
    let api = TitenApi::new(&config);

    match action {
        ScheduleAction::List { account, status } => {
            let mut path = "/api/schedules?".to_string();
            if let Some(a) = account {
                path.push_str(&format!("account_id={a}&"));
            }
            if let Some(s) = status {
                path.push_str(&format!("status={s}&"));
            }
            let resp = api.get(&path).await?;
            print_data(&resp);
        }
        ScheduleAction::Create {
            account,
            text,
            at,
            media_type,
        } => {
            let body = json!({
                "account_id": account,
                "caption": text,
                "scheduled_at": at,
                "media_type": media_type.unwrap_or_else(|| "TEXT".into()),
            });
            let resp = api.post("/api/schedules", body).await?;
            print_data(&resp);
        }
        ScheduleAction::Cancel { id } => {
            let resp = api.delete(&format!("/api/schedules/{id}")).await?;
            print_data(&resp);
        }
        ScheduleAction::Upcoming => {
            let resp = api.get("/api/schedules/upcoming").await?;
            print_data(&resp);
        }
    }
    Ok(())
}
