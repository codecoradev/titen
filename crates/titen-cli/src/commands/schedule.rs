use anyhow::Result;
use clap::Subcommand;

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
    match action {
        ScheduleAction::List { account, status } => {
            println!("Listing schedules...");
            if let Some(a) = account {
                println!("  Account: {a}");
            }
            if let Some(s) = status {
                println!("  Status: {s}");
            }
        }
        ScheduleAction::Create {
            account,
            text,
            at,
            media_type,
        } => {
            println!("Scheduling post on account: {account}");
            println!("  At: {at}");
            println!("  Type: {media_type:?}");
            println!("  Text: {text}");
        }
        ScheduleAction::Cancel { id } => {
            println!("Canceling schedule: {id}");
        }
        ScheduleAction::Upcoming => {
            println!("Showing upcoming schedules...");
        }
    }
    Ok(())
}
