//! Centralized publish orchestration (#250 follow-up, thread-bundle Phase 1).
//!
//! One place maps a publish request (media type + inputs) onto the
//! `ThreadsClient` primitives. `posts.rs` (instant publish), `scheduler.rs`
//! (scheduled publish) and the upcoming thread-bundle executor all call this
//! instead of maintaining their own `match media_type` blocks.

use crate::threads_client::ThreadsClient;
use crate::{Result, TitenError, models::Account};

/// Everything needed to publish exactly one Threads post.
#[derive(Debug, Clone, Default)]
pub struct PublishRequest {
    /// `TEXT`, `IMAGE`, `VIDEO`, or `CAROUSEL`.
    pub media_type: String,
    pub caption: Option<String>,
    /// IMAGE: single URL. VIDEO: single URL. CAROUSEL: 2–20 URLs.
    pub media_urls: Vec<String>,
    pub alt_text: Option<String>,
    pub location_id: Option<String>,
    /// Publish as a reply to this existing Threads post ID.
    pub reply_to_id: Option<String>,
}

impl PublishRequest {
    /// Build from a stored `Schedule` row (media_urls is a JSON string there).
    pub fn from_schedule(
        media_type: &str,
        caption: Option<&str>,
        media_urls_json: Option<&str>,
        location_id: Option<&str>,
        reply_to_id: Option<&str>,
    ) -> Self {
        let media_urls: Vec<String> = media_urls_json
            .and_then(|u| serde_json::from_str(u).ok())
            .unwrap_or_default();
        Self {
            media_type: media_type.to_string(),
            caption: caption.map(|s| s.to_string()),
            media_urls,
            alt_text: None,
            location_id: location_id.map(|s| s.to_string()),
            reply_to_id: reply_to_id.map(|s| s.to_string()),
        }
    }

    fn image_url(&self) -> Option<&str> {
        self.media_urls.first().map(|s| s.as_str())
    }

    fn video_url(&self) -> Option<&str> {
        self.media_urls.first().map(|s| s.as_str())
    }
}

/// Validate publish-request invariants (shared by API handlers & scheduler).
///
/// Returns `Err(message)` when the request cannot be published.
pub fn validate(req: &PublishRequest) -> Result<()> {
    match req.media_type.as_str() {
        "TEXT" => {}
        "IMAGE" => {
            let url = req.image_url().filter(|u| !u.is_empty()).ok_or_else(|| {
                TitenError::InvalidRequest("image_url is required for IMAGE posts".to_string())
            })?;
            validate_media_url(url)?;
        }
        "VIDEO" => {
            let url = req.video_url().filter(|u| !u.is_empty()).ok_or_else(|| {
                TitenError::InvalidRequest("video_url is required for VIDEO posts".to_string())
            })?;
            validate_media_url(url)?;
        }
        "CAROUSEL" => {
            if req.media_urls.len() < 2 || req.media_urls.len() > 20 {
                return Err(TitenError::InvalidRequest(format!(
                    "CAROUSEL requires 2-20 image_urls, got {}",
                    req.media_urls.len()
                )));
            }
            for url in &req.media_urls {
                validate_media_url(url)?;
            }
        }
        other => {
            return Err(TitenError::InvalidRequest(format!(
                "Unsupported media type: {other}"
            )));
        }
    }
    Ok(())
}

/// SSRF / scheme validation shared with the API layer (#P3.8 rules).
pub fn validate_media_url(url: &str) -> Result<()> {
    if !url.starts_with("https://") {
        return Err(TitenError::InvalidRequest(format!(
            "media URL must be https, got: {url}"
        )));
    }
    Ok(())
}

/// Publish exactly one post (or reply) through the shared client.
///
/// Reply support is media-type agnostic: `reply_to_id` composes with
/// TEXT/IMAGE/VIDEO/CAROUSEL alike (Threads API accepts `reply_to_id` on any
/// container), matching the official publishing reference.
pub async fn publish(
    client: &ThreadsClient,
    account: &Account,
    req: &PublishRequest,
) -> Result<String> {
    validate(req)?;
    let reply_to = req.reply_to_id.as_deref();
    match req.media_type.as_str() {
        "TEXT" => {
            client
                .publish_text(
                    account,
                    req.caption.as_deref().unwrap_or(""),
                    req.location_id.as_deref(),
                    reply_to,
                )
                .await
        }
        "IMAGE" => {
            client
                .publish_image(
                    account,
                    req.caption.as_deref(),
                    req.image_url().unwrap_or_default(),
                    req.alt_text.as_deref(),
                    req.location_id.as_deref(),
                    reply_to,
                )
                .await
        }
        "VIDEO" => {
            client
                .publish_video(
                    account,
                    req.caption.as_deref(),
                    req.video_url().unwrap_or_default(),
                    req.location_id.as_deref(),
                    reply_to,
                )
                .await
        }
        "CAROUSEL" => {
            let mut children_ids = Vec::with_capacity(req.media_urls.len());
            for url in &req.media_urls {
                match client
                    .create_carousel_item(account, "IMAGE", Some(url.as_str()), None, None)
                    .await
                {
                    Ok(id) => children_ids.push(id),
                    Err(e) => {
                        tracing::error!(
                            "Partial carousel failure after {n} children. \
                             Orphaned children IDs (manual cleanup needed): {children_ids:?}",
                            n = children_ids.len()
                        );
                        return Err(TitenError::InvalidRequest(format!(
                            "Failed to create carousel item: {e}"
                        )));
                    }
                }
            }
            client
                .publish_carousel(account, req.caption.as_deref(), &children_ids, reply_to)
                .await
        }
        other => Err(TitenError::InvalidRequest(format!(
            "Unsupported media type: {other}"
        ))),
    }
}
