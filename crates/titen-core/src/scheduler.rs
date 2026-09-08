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

    // Thread-bundle Phase 2: promote bundle members whose ROOT item (seq 0)
    // is due. Members 1..n enter as 'bundle_waiting' and only become
    // 'pending' once the previous item publishes — this keeps the chain
    // ordered while everything shares the existing schedules machinery.
    if let Err(e) = store.promote_due_bundle_roots().await {
        error!("Failed to promote due bundle roots: {e}");
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

        // Unified publish path (thread-bundle Phase 1): replies (any media
        // type) and root posts share the same Publisher — the only difference
        // is the optional reply_to_id. Produces the same result envelope as
        // the previous per-media match so the shared bookkeeping applies.
        let mut req = crate::publisher::PublishRequest::from_schedule(
            &schedule.media_type,
            schedule.caption.as_deref(),
            schedule.media_urls.as_deref(),
            schedule.location_id.as_deref(),
            schedule.reply_to_id.as_deref(),
        );
        let mut reply_to = req
            .reply_to_id
            .take()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        // Bundle chain markers ("bundle:<bundle_id>:<seq>") resolve to the
        // real Threads post id of the referenced bundle member.
        if let Some(marker) = reply_to.clone() {
            if let Some(Some(target_id)) = resolve_bundle_marker(store, &marker).await {
                reply_to = Some(target_id);
            }
        }
        req.reply_to_id = reply_to.clone();

        let result = match crate::publisher::publish(client, &account, &req).await {
            Ok(post_id) => {
                let _ = store.track_rate(&schedule.account_id, "post").await;
                match reply_to {
                    Some(rt) => Ok(serde_json::json!({
                        "threads_post_id": post_id,
                        "reply_to_id": rt,
                    })),
                    None => Ok(serde_json::json!({ "threads_post_id": post_id })),
                }
            }
            Err(e) => Err(e.to_string()),
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
                    reply_to_id: schedule.reply_to_id.clone(),
                };
                // Fetch permalink best-effort (non-fatal if it fails).
                let permalink = client
                    .get_permalink(&account, &post_id)
                    .await
                    .ok()
                    .flatten();
                let _ = store
                    .create_post_with_threads_id(
                        &post_id_uuid,
                        &create_post,
                        &post_id,
                        permalink.as_deref(),
                    )
                    .await;

                // Mark schedule as published
                let _ = store
                    .update_schedule_status(&schedule.id, "published", Some(&result_str), None)
                    .await;

                // Thread-bundle: promote the next waiting member of this bundle.
                if let Some(ref bundle_id) = schedule.bundle_id {
                    let seq = schedule.bundle_seq.unwrap_or(0);
                    match store.promote_next_bundle_item(bundle_id, seq).await {
                        Ok(n) if n > 0 => {
                            info!("Bundle {bundle_id}: promoted {n} item(s) after seq {seq}");
                        }
                        Ok(_) => {}
                        Err(e) => error!("Bundle {bundle_id} chain advance failed: {e}"),
                    }
                }

                info!(
                    "Schedule {} published as post {} for @{}",
                    schedule.id, post_id, account.username
                );
            }
            Err(e) => {
                let _ = store
                    .update_schedule_status(&schedule.id, "failed", None, Some(&e))
                    .await;
                // Thread-bundle: a failed chain item strands the rest — fail
                // every remaining waiting member so nothing hangs forever.
                if let Some(ref bundle_id) = schedule.bundle_id {
                    match store
                        .fail_remaining_bundle(
                            bundle_id,
                            &format!(
                                "bundle chain stopped at seq {}: {e}",
                                schedule.bundle_seq.unwrap_or(0)
                            ),
                        )
                        .await
                    {
                        Ok(n) if n > 0 => {
                            warn!(
                                "Bundle {bundle_id}: failed {n} remaining item(s) after chain break"
                            );
                        }
                        Ok(_) => {}
                        Err(e2) => error!("Bundle {bundle_id} cascade fail failed: {e2}"),
                    }
                }
                error!("Schedule {} failed: {e}", schedule.id);
            }
        }
    }

    Ok(())
}

/// Resolve a `bundle:<bundle_id>:<seq>` reply marker to the referenced
/// member's published Threads post id.
/// Returns:
/// - `Some(Some(post_id))` — referenced member published; proceed.
/// - `Some(None)` — referenced member failed; the caller should fail this
///   item (its reply target can never exist).
/// - `None` — not a bundle marker.
async fn resolve_bundle_marker(store: &Store, marker: &str) -> Option<Option<String>> {
    let rest = marker.strip_prefix("bundle:")?;
    let (bundle_id, seq_str) = rest.rsplit_once(':')?;
    let seq: i64 = seq_str.parse().ok()?;
    let members = store.get_bundle_schedules(bundle_id).await.ok()?;
    let target = members.iter().find(|s| s.bundle_seq == Some(seq))?;
    match target.status.as_str() {
        "published" => Some(target.result_post_id.clone()),
        "failed" | "cancelled" | "rejected" => Some(None),
        _ => Some(None),
    }
}
