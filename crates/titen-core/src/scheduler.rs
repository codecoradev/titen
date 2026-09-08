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
    // Reconcile: promote any waiting member whose predecessor published,
    // regardless of how publication happened (covers lost promote hooks,
    // hook errors, and crashes between publish and promote).
    if let Err(e) = store.reconcile_bundle_promotions().await {
        error!("Failed to reconcile bundle promotions: {e}");
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
                cascade_fail_bundle(store, &schedule, &format!("account not found: {e}")).await;
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
            cascade_fail_bundle(store, &schedule, "account is inactive").await;
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
            if marker.starts_with("bundle:") {
                match resolve_bundle_marker(store, &marker).await {
                    Ok(Some(Some(target_id))) => {
                        reply_to = Some(target_id);
                    }
                    // Target failed or not yet published / malformed marker:
                    // fail this item cleanly instead of sending a literal
                    // marker to Threads.
                    Ok(Some(None)) | Ok(None) => {
                        // Ok(None): malformed marker — permanent problem,
                        // fail the item instead of publishing garbage.
                        let msg = format!("bundle reply target unavailable: {marker}");
                        let _ = store
                            .update_schedule_status(&schedule.id, "failed", None, Some(&msg))
                            .await;
                        cascade_fail_bundle(store, &schedule, &msg).await;
                        continue;
                    }
                    Err(e) => {
                        // Transient store error (lock timeout, pool
                        // exhaustion): NOT fatal — reset to pending so the
                        // next tick retries the lookup instead of destroying
                        // publishable content.
                        warn!(
                            "Bundle marker lookup errored for schedule {} — retrying next tick: {e}",
                            schedule.id
                        );
                        let _ = store
                            .update_schedule_status(&schedule.id, "pending", None, None)
                            .await;
                        continue;
                    }
                }
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

/// Fail every remaining waiting member of a bundle when one of its members
/// fails outside the publish-result path (account missing/inactive, HITL
/// reject, etc.). No-op for non-bundle schedules.
async fn cascade_fail_bundle(store: &Store, schedule: &crate::models::Schedule, reason: &str) {
    if let Some(ref bundle_id) = schedule.bundle_id {
        match store
            .fail_remaining_bundle(bundle_id, &format!("bundle chain stopped: {reason}"))
            .await
        {
            Ok(n) if n > 0 => {
                warn!("Bundle {bundle_id}: failed {n} remaining item(s) — {reason}");
            }
            Ok(_) => {}
            Err(e) => error!("Bundle {bundle_id} cascade fail failed: {e}"),
        }
    }
}

/// Resolve a `bundle:<bundle_id>:<seq>` reply marker to the referenced
/// member's published Threads post id.
/// Returns:
/// - `Ok(Some(Some(post_id)))` — referenced member published; proceed.
/// - `Ok(Some(None))` — referenced member failed/cancelled; its reply can
///   never exist, so the caller should fail this item permanently.
/// - `Ok(None)` — malformed marker; permanent, fail the item.
/// - `Err(e)` — transient store error; the caller should retry next tick
///   instead of destroying scheduled content.
async fn resolve_bundle_marker(
    store: &Store,
    marker: &str,
) -> crate::Result<Option<Option<String>>> {
    // Malformed markers and missing members are PERMANENT (client-supplied
    // input) -> Ok(None): the caller fails the item. Only store errors are
    // Err: transient, the caller retries next tick.
    let rest = match marker.strip_prefix("bundle:") {
        Some(r) => r,
        None => return Ok(None), // unreachable — caller pre-gates on prefix
    };
    let (bundle_id, seq_str) = match rest.rsplit_once(':') {
        Some(pair) => pair,
        None => return Ok(None),
    };
    let seq: i64 = match seq_str.parse() {
        Ok(s) => s,
        Err(_) => return Ok(None),
    };
    let members = store.get_bundle_schedules(bundle_id).await?;
    let target = match members.iter().find(|s| s.bundle_seq == Some(seq)) {
        Some(t) => t,
        // Referenced member does not exist (deleted bundle, bad index):
        // permanent — its post id can never appear.
        None => return Ok(None),
    };
    match target.status.as_str() {
        "published" => Ok(Some(target.result_post_id.clone())),
        "failed" | "cancelled" | "rejected" => Ok(Some(None)),
        _ => Ok(Some(None)),
    }
}
