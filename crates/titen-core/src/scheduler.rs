use crate::error::Result;
use crate::store::Store;
use crate::threads_client::ThreadsClient;
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{debug, error, info, warn};

/// Scheduler that ticks every N seconds to check for due schedules
pub struct TitenScheduler {
    scheduler: JobScheduler,
    store: Arc<Store>,
    threads_client: Arc<ThreadsClient>,
    interval_secs: u64,
}

impl TitenScheduler {
    /// Create and start the scheduler
    pub async fn new(store: Arc<Store>, threads_client: Arc<ThreadsClient>) -> Result<Self> {
        let interval_secs: u64 = std::env::var("TITEN_SCHEDULER_INTERVAL_SECS")
            .unwrap_or_else(|_| "60".to_string())
            .parse()
            .unwrap_or(60);

        let sched = JobScheduler::new().await.map_err(|e| {
            crate::error::TitenError::ConfigError(format!("Failed to create scheduler: {e}"))
        })?;

        // Schedule posting tick — runs every N seconds
        let _store_tick = store.clone();
        let _client_tick = threads_client.clone();
        sched
            .add(
                Job::new_repeated_async(
                    std::time::Duration::from_secs(interval_secs),
                    move |_uuid, _l| {
                        let store = _store_tick.clone();
                        let client = _client_tick.clone();
                        Box::pin(async move {
                            if let Err(e) = process_due_schedules(&store, &client).await {
                                error!("Scheduler tick error: {e}");
                            }
                        })
                    },
                )
                .map_err(|e| {
                    crate::error::TitenError::ConfigError(format!(
                        "Failed to create schedule job: {e}"
                    ))
                })?,
            )
            .await
            .map_err(|e| {
                crate::error::TitenError::ConfigError(format!("Failed to schedule job: {e}"))
            })?;

        // Token check every 6 hours
        let store_token = store.clone();
        let client_token = threads_client.clone();
        sched
            .add(
                Job::new_repeated_async(
                    std::time::Duration::from_secs(6 * 3600),
                    move |_uuid, _l| {
                        let _store = store_token.clone();
                        let client = client_token.clone();
                        Box::pin(async move {
                            info!("Running token expiry check...");
                            let results = client.check_all_tokens().await;
                            for (username, status) in results {
                                info!("Token check: @{username} → {status}");
                            }
                        })
                    },
                )
                .map_err(|e| {
                    crate::error::TitenError::ConfigError(format!(
                        "Failed to create token check job: {e}"
                    ))
                })?,
            )
            .await
            .map_err(|e| {
                crate::error::TitenError::ConfigError(format!(
                    "Failed to schedule token check: {e}"
                ))
            })?;

        Ok(Self {
            scheduler: sched,
            store,
            threads_client,
            interval_secs,
        })
    }

    /// Start the scheduler (non-blocking)
    pub async fn start(&self) -> Result<()> {
        info!(
            "Starting titen scheduler (interval: {}s)",
            self.interval_secs
        );
        self.scheduler.start().await.map_err(|e| {
            crate::error::TitenError::ConfigError(format!("Failed to start scheduler: {e}"))
        })?;
        Ok(())
    }

    /// Process all due schedules right now (for testing/manual trigger)
    pub async fn tick_now(&self) -> Result<()> {
        process_due_schedules(&self.store, &self.threads_client).await
    }
}

/// Process all due schedules — called by the scheduler tick
async fn process_due_schedules(store: &Store, client: &ThreadsClient) -> Result<()> {
    // B4 fix: Reap schedules stuck in 'processing' (server crash recovery).
    // Any row processing > 5 minutes is considered stale → reset to 'pending'.
    match store.reap_stale_schedules(300).await {
        Ok(reaped) if reaped > 0 => {
            warn!("Reaped {reaped} stale schedule(s) stuck in 'processing'");
        }
        Ok(_) => {}
        Err(e) => {
            error!("Failed to reap stale schedules: {e}");
        }
    }

    let due_schedules = match store.get_due_schedules().await {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to get due schedules: {e}");
            return Err(e);
        }
    };

    if due_schedules.is_empty() {
        return Ok(());
    }

    info!("Processing {} due schedule(s)", due_schedules.len());

    for schedule in due_schedules {
        // Atomically claim: pending → processing (prevents double-post in HA)
        match store.claim_schedule(&schedule.id).await {
            Ok(true) => {} // claimed successfully
            Ok(false) => {
                debug!("Schedule {} already claimed by another worker", schedule.id);
                continue;
            }
            Err(e) => {
                error!("Failed to claim schedule {}: {e}", schedule.id);
                continue;
            }
        }

        // Get account
        let account = match store.get_account(&schedule.account_id).await {
            Ok(a) => a,
            Err(e) => {
                error!(
                    "Account {} not found for schedule {}: {e}",
                    schedule.account_id, schedule.id
                );
                let _ = store
                    .update_schedule_status(
                        &schedule.id,
                        "failed",
                        None,
                        Some(&format!("Account not found: {e}")),
                    )
                    .await;
                continue;
            }
        };

        // #117 fix: Skip schedules for inactive accounts
        if !account.is_active {
            warn!(
                "Schedule {} skipped — account @{} is inactive",
                schedule.id, account.username
            );
            let _ = store
                .update_schedule_status(&schedule.id, "failed", None, Some("Account is inactive"))
                .await;
            continue;
        }

        // Check token is still valid — auto-refresh if expiring
        let account = match account.token_status() {
            "valid" => account,
            "expiring_soon" | "expired" => {
                info!(
                    "Token {} for @{} — auto-refreshing before publish",
                    account.token_status(),
                    account.username
                );
                match client.ensure_valid_token(&account).await {
                    Ok(refreshed) => {
                        info!("Token refreshed for @{}", refreshed.username);
                        refreshed
                    }
                    Err(e) => {
                        let _ = store
                            .update_schedule_status(
                                &schedule.id,
                                "failed",
                                None,
                                Some(&format!("Token refresh failed: {e}")),
                            )
                            .await;
                        warn!(
                            "Schedule {} skipped — token refresh failed for @{}: {e}",
                            schedule.id, account.username
                        );
                        continue;
                    }
                }
            }
            _ => account, // "unknown" — attempt anyway
        };

        // Check rate limit
        let remaining = match store
            .check_rate_limit(&schedule.account_id, "post", 250)
            .await
        {
            Ok(r) => r,
            Err(_) => {
                let _ = store
                    .update_schedule_status(
                        &schedule.id,
                        "failed",
                        None,
                        Some("Rate limit exceeded"),
                    )
                    .await;
                continue;
            }
        };

        if remaining == 0 {
            let _ = store
                .update_schedule_status(&schedule.id, "failed", None, Some("Rate limit exceeded"))
                .await;
            warn!(
                "Schedule {} skipped — rate limit for @{}",
                schedule.id, account.username
            );
            continue;
        }

        // Publish based on media type
        let result = match schedule.media_type.as_str() {
            "TEXT" => {
                let caption = schedule.caption.as_deref().unwrap_or("");
                match client
                    .publish_text(&account, caption, schedule.location_id.as_deref())
                    .await
                {
                    Ok(post_id) => {
                        let _ = store.track_rate(&schedule.account_id, "post").await;
                        Ok(serde_json::json!({ "threads_post_id": post_id }))
                    }
                    Err(e) => Err(e.to_string()),
                }
            }
            "IMAGE" => {
                let urls: Vec<String> = schedule
                    .media_urls
                    .as_ref()
                    .and_then(|u| serde_json::from_str(u).ok())
                    .unwrap_or_default();
                let image_url = urls.first().cloned().unwrap_or_default();
                if image_url.is_empty() {
                    Err("No image URL provided".to_string())
                } else {
                    match client
                        .publish_image(
                            &account,
                            schedule.caption.as_deref(),
                            &image_url,
                            None,
                            schedule.location_id.as_deref(),
                        )
                        .await
                    {
                        Ok(post_id) => {
                            let _ = store.track_rate(&schedule.account_id, "post").await;
                            Ok(serde_json::json!({ "threads_post_id": post_id }))
                        }
                        Err(e) => Err(e.to_string()),
                    }
                }
            }
            "VIDEO" => {
                let urls: Vec<String> = schedule
                    .media_urls
                    .as_ref()
                    .and_then(|u| serde_json::from_str(u).ok())
                    .unwrap_or_default();
                let video_url = urls.first().cloned().unwrap_or_default();
                if video_url.is_empty() {
                    Err("No video URL provided".to_string())
                } else {
                    match client
                        .publish_video(
                            &account,
                            schedule.caption.as_deref(),
                            &video_url,
                            schedule.location_id.as_deref(),
                        )
                        .await
                    {
                        Ok(post_id) => {
                            let _ = store.track_rate(&schedule.account_id, "post").await;
                            Ok(serde_json::json!({ "threads_post_id": post_id }))
                        }
                        Err(e) => Err(e.to_string()),
                    }
                }
            }
            "CAROUSEL" => {
                let urls: Vec<String> = schedule
                    .media_urls
                    .as_ref()
                    .and_then(|u| serde_json::from_str(u).ok())
                    .unwrap_or_default();
                if urls.len() < 2 || urls.len() > 20 {
                    Err(format!(
                        "CAROUSEL requires 2-20 image_urls, got {}",
                        urls.len()
                    ))
                } else {
                    // Create child containers, then publish carousel
                    let mut children_ids = Vec::with_capacity(urls.len());
                    let mut had_error = None;
                    for url in &urls {
                        match client
                            .create_carousel_item(&account, "IMAGE", Some(url.as_str()), None, None)
                            .await
                        {
                            Ok(id) => children_ids.push(id),
                            Err(e) => {
                                error!(
                                    "Partial carousel failure after {n} children. \
                                     Orphaned children IDs (manual cleanup needed): {children_ids:?}",
                                    n = children_ids.len()
                                );
                                had_error = Some(e.to_string());
                                break;
                            }
                        }
                    }
                    match had_error {
                        Some(e) => Err(format!("Failed to create carousel item: {e}")),
                        None => match client
                            .publish_carousel(&account, schedule.caption.as_deref(), &children_ids)
                            .await
                        {
                            Ok(post_id) => {
                                let _ = store.track_rate(&schedule.account_id, "post").await;
                                Ok(serde_json::json!({ "threads_post_id": post_id }))
                            }
                            Err(e) => Err(e.to_string()),
                        },
                    }
                }
            }
            _ => Err(format!("Unsupported media type: {}", schedule.media_type)),
        };

        match result {
            Ok(result_json) => {
                let result_str = serde_json::to_string(&result_json).unwrap_or_default();
                let post_id = result_json
                    .get("threads_post_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                // Create post record with threads_post_id (#109 fix)
                let post_id_uuid = uuid::Uuid::now_v7().to_string();
                let create_post = crate::models::CreatePost {
                    account_id: schedule.account_id.clone(),
                    media_type: Some(schedule.media_type.clone()),
                    caption: schedule.caption.clone(),
                    text_attachment: schedule.text_attachment.clone(),
                    image_url: None,
                    video_url: None,
                    image_urls: None,
                    media_ids: None,
                    alt_text: None,
                };
                let _ = store
                    .create_post_with_threads_id(&post_id_uuid, &create_post, &post_id)
                    .await;

                // Mark schedule as published
                let _ = store
                    .update_schedule_status(&schedule.id, "published", Some(&result_str), None)
                    .await;

                info!(
                    "Schedule {} published as post {} for @{}",
                    schedule.id, post_id, account.username
                );
            }
            Err(e) => {
                let _ = store
                    .update_schedule_status(&schedule.id, "failed", None, Some(&e))
                    .await;
                error!("Schedule {} failed: {e}", schedule.id);
            }
        }
    }

    Ok(())
}
