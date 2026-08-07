use crate::error::Result;
use crate::models::{
    CommentData, ContainerStatus, InsightMetric, PublishingLimit, UserInsightMetric, UserProfile,
};
use crate::store::Store;
use reqwest::Client;
use tracing::{info, warn};

/// Official Threads Graph API base URL.
///
/// Can also use `graph.threads.com` — both are equivalent per Meta docs.
const THREADS_GRAPH_API: &str = "https://graph.threads.net";

/// Threads API client for interacting with the official Threads Graph API.
///
/// Reference: <https://developers.facebook.com/documentation/threads/reference>
pub struct ThreadsClient {
    http: Client,
    store: std::sync::Arc<Store>,
}

impl ThreadsClient {
    pub fn new(store: std::sync::Arc<Store>) -> Self {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .build()
            .unwrap_or_else(|e| {
                tracing::error!(
                    "Failed to build HTTP client with timeouts, falling back to default: {e}"
                );
                Client::new()
            });
        Self { http, store }
    }

    // ─── Internal Helpers ─────────────────────────────────────

    /// Send a request and parse JSON, checking for Threads API errors.
    ///
    /// Threads API returns HTTP 4xx/5xx with a JSON error body like:
    /// `{"error": {"message": "...", "type": "OAuthException", "code": 190}}`
    ///
    /// reqwest's `.send()` does NOT error on non-2xx responses, so we must
    /// explicitly check the status and extract the error message.
    async fn threads_request(
        &self,
        method: reqwest::Method,
        url: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let mut req = self.http.request(method, url);
        if let Some(json_body) = body {
            req = req.json(json_body);
        }
        let resp = req.send().await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await.map_err(|e| {
            crate::error::TitenError::ThreadsApiError(format!(
                "Failed to parse Threads API response: {e}"
            ))
        })?;

        if !status.is_success() {
            // Extract error message from standard Threads/Meta error format
            let msg = json
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown Threads API error");
            let code = json
                .get("error")
                .and_then(|e| e.get("code"))
                .and_then(|c| c.as_i64())
                .unwrap_or(0);
            let etype = json
                .get("error")
                .and_then(|e| e.get("type"))
                .and_then(|t| t.as_str())
                .unwrap_or("Unknown");
            return Err(crate::error::TitenError::ThreadsApiError(format!(
                "{msg} [{etype} #{code}] (HTTP {status})"
            )));
        }

        Ok(json)
    }

    /// GET wrapper for threads_request.
    async fn threads_get(&self, url: &str) -> Result<serde_json::Value> {
        self.threads_request(reqwest::Method::GET, url, None).await
    }

    /// POST wrapper for threads_request.
    async fn threads_post(&self, url: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
        self.threads_request(reqwest::Method::POST, url, Some(body))
            .await
    }

    // ─── Token Management ─────────────────────────────────────

    /// Ensure the account's token is valid, refreshing if expiring.
    ///
    /// Call this before any API request. If the token is expiring within
    /// 7 days or expired, it attempts to refresh automatically.
    /// Returns the (possibly updated) account.
    pub async fn ensure_valid_token(
        &self,
        account: &crate::models::Account,
    ) -> Result<crate::models::Account> {
        match account.token_status() {
            "valid" => Ok(account.clone()),
            "expiring_soon" | "expired" => self.refresh_token(account).await,
            _ => Ok(account.clone()),
        }
    }

    /// Exchange a short-lived token for a long-lived token.
    ///
    /// `GET /access_token?grant_type=th_exchange_token&client_secret={secret}&access_token={token}`
    ///
    /// Returns the new long-lived access_token + expires_in seconds.
    pub async fn exchange_long_lived_token(
        &self,
        short_lived_token: &str,
        app_secret: &str,
    ) -> Result<(String, i64)> {
        let url = format!(
            "{THREADS_GRAPH_API}/access_token?grant_type=th_exchange_token&client_secret={app_secret}&access_token={short_lived_token}"
        );

        let resp: serde_json::Value =
            self.http
                .get(&url)
                .send()
                .await?
                .json()
                .await
                .map_err(|e| {
                    crate::error::TitenError::ThreadsApiError(format!(
                        "Failed to parse token exchange response: {e}"
                    ))
                })?;

        let access_token = resp
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                crate::error::TitenError::ThreadsApiError(
                    "No access_token in exchange response".to_string(),
                )
            })?
            .to_string();

        let expires_in = resp
            .get("expires_in")
            .and_then(|v| v.as_i64())
            .unwrap_or(60 * 24 * 3600);

        Ok((access_token, expires_in))
    }

    /// Exchange an OAuth authorization code for a short-lived access token.
    ///
    /// `POST https://graph.threads.net/oauth/access_token`
    ///
    /// Returns the short-lived access_token + user_id from Meta.
    pub async fn exchange_code_for_token(
        &self,
        code: &str,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
    ) -> Result<(String, String)> {
        let url = format!("{THREADS_GRAPH_API}/oauth/access_token");

        let params = [
            ("grant_type", "authorization_code"),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code),
            ("redirect_uri", redirect_uri),
        ];

        let resp = self.http.post(&url).form(&params).send().await?;

        let status = resp.status();
        let body: serde_json::Value = resp.json().await.map_err(|e| {
            crate::error::TitenError::ThreadsApiError(format!(
                "Failed to parse code exchange response (HTTP {}): {e}",
                status
            ))
        })?;

        // Check for Threads/Meta OAuth error response BEFORE looking for access_token
        if let Some(err) = body.get("error") {
            let msg = err
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown OAuth error");
            let fb_type = err
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("OAuthException");
            let fb_code = err.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
            return Err(crate::error::TitenError::ThreadsApiError(format!(
                "Threads API error (HTTP {status}): {msg} [{fb_type} #{fb_code}]"
            )));
        }

        let access_token = body
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                crate::error::TitenError::ThreadsApiError(format!(
                    "No access_token in code exchange response (HTTP {status}): {body}"
                ))
            })?
            .to_string();

        let user_id = body
            .get("user_id")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| {
                tracing::warn!("No user_id in token exchange response, deriving from token");
                ""
            })
            .to_string();

        // If user_id is empty, try to extract from access_token (JWT format)
        let user_id = if user_id.is_empty() {
            // Try to decode user_id from the token (Meta tokens sometimes embed it)
            access_token.split('|').next().unwrap_or("").to_string()
        } else {
            user_id
        };

        Ok((access_token, user_id))
    }

    /// Resolve account info (user_id + username) from a Threads access token.
    ///
    /// Calls `GET /me?fields=id,username` to auto-discover the account identity.
    pub async fn resolve_account(&self, access_token: &str) -> Result<(String, String)> {
        let url =
            format!("{THREADS_GRAPH_API}/v1.0/me?fields=id,username&access_token={access_token}");

        let resp: serde_json::Value =
            self.http
                .get(&url)
                .send()
                .await?
                .json()
                .await
                .map_err(|e| {
                    crate::error::TitenError::ThreadsApiError(format!(
                        "Failed to resolve account: {e}"
                    ))
                })?;

        let user_id = resp
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                crate::error::TitenError::ThreadsApiError("No id in /me response".to_string())
            })?
            .to_string();

        let username = resp
            .get("username")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                crate::error::TitenError::ThreadsApiError("No username in /me response".to_string())
            })?
            .to_string();

        Ok((user_id, username))
    }

    /// Refresh an account's access token via the Threads API.
    ///
    /// `GET /refresh_access_token?grant_type=th_refresh_token&access_token={token}`
    ///
    /// No separate refresh token needed — Threads uses the current access_token.
    pub async fn refresh_token(
        &self,
        account: &crate::models::Account,
    ) -> Result<crate::models::Account> {
        let url = format!(
            "{THREADS_GRAPH_API}/refresh_access_token?grant_type=th_refresh_token&access_token={}",
            account.access_token
        );

        let resp: serde_json::Value =
            self.http
                .get(&url)
                .send()
                .await?
                .json()
                .await
                .map_err(|e| {
                    crate::error::TitenError::ThreadsApiError(format!(
                        "Failed to parse refresh response: {e}"
                    ))
                })?;

        let new_token = resp
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                crate::error::TitenError::TokenRefreshFailed(
                    "No access_token in refresh response".to_string(),
                )
            })?;

        let expires_in = resp
            .get("expires_in")
            .and_then(|v| v.as_i64())
            .unwrap_or(60 * 24 * 3600); // default ~60 days

        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(expires_in);
        let expires_at_str = expires_at.to_rfc3339();

        info!(
            "Token refreshed for {}: expires {}",
            account.username, expires_at_str
        );
        let update = crate::models::UpdateAccount {
            access_token: Some(new_token.to_string()),
            expires_at: Some(expires_at_str),
            is_active: None,
        };

        let account = self.store.update_account(&account.id, &update).await?;
        Ok(account)
    }

    // ─── Publishing ───────────────────────────────────────────

    /// Create a Threads media container (first step for all posts).
    ///
    /// Official: `POST /v1.0/{threads_user_id}/threads`
    ///
    /// This is the generic container creation that supports:
    /// - Text posts (`media_type=TEXT`)
    /// - Image posts (`media_type=IMAGE` + `image_url`)
    /// - Video posts (`media_type=VIDEO` + `video_url`)
    /// - Replies (`reply_to_id` parameter)
    /// - Topic tags (`topic_tag` parameter)
    /// - Link attachments (`link_attachment` parameter, text-only)
    /// - GIF attachments (`gif_attachment` parameter, text-only)
    /// - Carousel items (`is_carousel_item=true`)
    /// - Reply control (`reply_control`)
    /// - Reply approvals (`enable_reply_approvals`)
    pub async fn create_container(
        &self,
        account: &crate::models::Account,
        media_type: &str,
        text: Option<&str>,
        image_url: Option<&str>,
        video_url: Option<&str>,
    ) -> Result<String> {
        let mut body = serde_json::json!({
            "access_token": account.access_token,
            "media_type": media_type,
        });

        if let Some(t) = text {
            body["text"] = serde_json::json!(t);
        }
        if let Some(url) = image_url {
            body["image_url"] = serde_json::json!(url);
        }
        if let Some(url) = video_url {
            body["video_url"] = serde_json::json!(url);
        }

        let url = format!("{THREADS_GRAPH_API}/v1.0/{}/threads", account.user_id);
        let resp = self.threads_post(&url, &body).await?;

        let container_id = resp.get("id").and_then(|v| v.as_str()).ok_or_else(|| {
            crate::error::TitenError::ThreadsApiError(format!(
                "No container id in response: {resp}"
            ))
        })?;

        Ok(container_id.to_string())
    }

    /// Create a Threads media container with full options.
    ///
    /// This is the extended version that supports all official parameters.
    pub async fn create_container_full(
        &self,
        account: &crate::models::Account,
        params: &ContainerParams,
    ) -> Result<String> {
        let mut body = serde_json::json!({
            "access_token": account.access_token,
            "media_type": params.media_type,
        });

        if let Some(t) = &params.text {
            body["text"] = serde_json::json!(t);
        }
        if let Some(url) = &params.image_url {
            body["image_url"] = serde_json::json!(url);
        }
        if let Some(url) = &params.video_url {
            body["video_url"] = serde_json::json!(url);
        }
        if let Some(tag) = &params.topic_tag {
            body["topic_tag"] = serde_json::json!(tag);
        }
        if let Some(link) = &params.link_attachment {
            body["link_attachment"] = serde_json::json!(link);
        }
        if let Some(gif) = &params.gif_attachment {
            body["gif_attachment"] = serde_json::json!(gif);
        }
        if let Some(id) = &params.reply_to_id {
            body["reply_to_id"] = serde_json::json!(id);
        }
        if let Some(rc) = &params.reply_control {
            body["reply_control"] = serde_json::json!(rc);
        }
        if let Some(carousel) = params.is_carousel_item {
            body["is_carousel_item"] = serde_json::json!(carousel);
        }
        if let Some(approvals) = params.enable_reply_approvals {
            body["enable_reply_approvals"] = serde_json::json!(approvals);
        }
        if let Some(children) = &params.children {
            body["children"] = serde_json::json!(children.join(","));
        }

        let url = format!("{THREADS_GRAPH_API}/v1.0/{}/threads", account.user_id);
        let resp = self.threads_post(&url, &body).await?;

        let container_id = resp.get("id").and_then(|v| v.as_str()).ok_or_else(|| {
            crate::error::TitenError::ThreadsApiError(format!(
                "No container id in response: {resp}"
            ))
        })?;

        Ok(container_id.to_string())
    }

    /// Publish a Threads media container (second step for all posts).
    ///
    /// Official: `POST /v1.0/{threads_user_id}/threads_publish`
    ///
    /// **Important:** It is recommended to wait ~30 seconds after creating
    /// a media container before publishing, to allow Threads servers to
    /// fully process the upload. Use [`check_container_status`] to verify.
    pub async fn publish_container(
        &self,
        account: &crate::models::Account,
        creation_id: &str,
    ) -> Result<String> {
        let url = format!(
            "{THREADS_GRAPH_API}/v1.0/{}/threads_publish",
            account.user_id
        );
        let body = serde_json::json!({
            "access_token": account.access_token,
            "creation_id": creation_id,
        });

        let resp = self.threads_post(&url, &body).await?;

        let post_id = resp.get("id").and_then(|v| v.as_str()).ok_or_else(|| {
            crate::error::TitenError::ThreadsApiError(format!(
                "No post id in publish response: {resp}"
            ))
        })?;

        Ok(post_id.to_string())
    }

    /// Check the status of a media container before publishing.
    ///
    /// Official: `GET /v1.0/{container_id}?fields=status`
    ///
    /// Returns `FINISHED` when the container is ready to be published.
    pub async fn check_container_status(
        &self,
        account: &crate::models::Account,
        container_id: &str,
    ) -> Result<ContainerStatus> {
        let url = format!(
            "{THREADS_GRAPH_API}/v1.0/{container_id}?fields=status&access_token={}",
            account.access_token
        );

        let resp = self.threads_get(&url).await?;

        Ok(ContainerStatus {
            id: container_id.to_string(),
            status: resp
                .get("status")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        })
    }

    // ─── Convenience Publishers ─────────────────────────────────

    /// Create and publish a text post (one-step convenience method).
    ///
    /// Internally: create container → publish.
    pub async fn publish_text(
        &self,
        account: &crate::models::Account,
        caption: &str,
    ) -> Result<String> {
        let container_id = self
            .create_container(account, "TEXT", Some(caption), None, None)
            .await?;
        let post_id = self.publish_container(account, &container_id).await?;
        Ok(post_id)
    }

    /// Create and publish an image post.
    pub async fn publish_image(
        &self,
        account: &crate::models::Account,
        caption: Option<&str>,
        image_url: &str,
        _alt_text: Option<&str>,
    ) -> Result<String> {
        let container_id = self
            .create_container(account, "IMAGE", caption, Some(image_url), None)
            .await?;
        let post_id = self.publish_container(account, &container_id).await?;
        Ok(post_id)
    }

    /// Create and publish a video post.
    ///
    /// Videos require processing time on Threads' side. This method
    /// creates the container, polls `check_container_status` until
    /// `FINISHED`, then publishes.
    pub async fn publish_video(
        &self,
        account: &crate::models::Account,
        caption: Option<&str>,
        video_url: &str,
    ) -> Result<String> {
        let container_id = self
            .create_container(account, "VIDEO", caption, None, Some(video_url))
            .await?;

        // Poll until container is ready (video processing)
        // Max ~90 attempts × 3s = ~4.5 min before giving up
        for attempt in 0..90 {
            let status = self.check_container_status(account, &container_id).await?;
            match status.status.as_deref() {
                Some("FINISHED") => break,
                Some("IN_PROGRESS") | None => {
                    if attempt == 89 {
                        return Err(crate::error::TitenError::ThreadsApiError(
                            "Video container processing timed out after ~4.5 minutes".to_string(),
                        ));
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                }
                Some(other) => {
                    return Err(crate::error::TitenError::ThreadsApiError(format!(
                        "Container processing failed: status={other}"
                    )));
                }
            }
        }

        let post_id = self.publish_container(account, &container_id).await?;
        Ok(post_id)
    }

    /// Create and publish a carousel post (3-step flow).
    ///
    /// Official flow:
    /// 1. Create N child containers with `is_carousel_item=true`
    /// 2. Create carousel container with `media_type=CAROUSEL` + `children`
    /// 3. Publish carousel container
    ///
    /// Limitations: 2–20 items per carousel.
    pub async fn publish_carousel(
        &self,
        account: &crate::models::Account,
        caption: Option<&str>,
        children_ids: &[String],
    ) -> Result<String> {
        if children_ids.len() < 2 {
            return Err(crate::error::TitenError::InvalidRequest(
                "Carousel requires at least 2 items".to_string(),
            ));
        }
        if children_ids.len() > 20 {
            return Err(crate::error::TitenError::InvalidRequest(
                "Carousel limited to 20 items".to_string(),
            ));
        }

        // Step 3: create carousel container with children
        let params = ContainerParams {
            media_type: "CAROUSEL".to_string(),
            text: caption.map(|s| s.to_string()),
            image_url: None,
            video_url: None,
            topic_tag: None,
            link_attachment: None,
            gif_attachment: None,
            reply_to_id: None,
            reply_control: None,
            is_carousel_item: None,
            enable_reply_approvals: None,
            children: Some(children_ids.to_vec()),
        };

        let carousel_container_id = self.create_container_full(account, &params).await?;
        let post_id = self
            .publish_container(account, &carousel_container_id)
            .await?;
        Ok(post_id)
    }

    /// Create a carousel child container.
    ///
    /// Use this for each image/video in a carousel, then pass all
    /// returned container IDs to [`publish_carousel`].
    pub async fn create_carousel_item(
        &self,
        account: &crate::models::Account,
        media_type: &str,
        image_url: Option<&str>,
        video_url: Option<&str>,
        alt_text: Option<&str>,
    ) -> Result<String> {
        let _alt_text = alt_text;
        let params = ContainerParams {
            media_type: media_type.to_string(),
            text: None,
            image_url: image_url.map(|s| s.to_string()),
            video_url: video_url.map(|s| s.to_string()),
            topic_tag: None,
            link_attachment: None,
            gif_attachment: None,
            reply_to_id: None,
            reply_control: None,
            is_carousel_item: Some(true),
            enable_reply_approvals: None,
            children: None,
        };

        self.create_container_full(account, &params).await
    }

    // ─── Post Deletion ─────────────────────────────────────────

    /// Delete a post on Threads.
    ///
    /// Official: `DELETE /v1.0/{threads_media_id}`
    pub async fn delete_post(
        &self,
        account: &crate::models::Account,
        threads_post_id: &str,
    ) -> Result<()> {
        let url = format!("{THREADS_GRAPH_API}/v1.0/{threads_post_id}");
        let body = serde_json::json!({
            "access_token": account.access_token,
        });

        let resp = self
            .http
            .delete(&url)
            .query(&body)
            .send()
            .await
            .map_err(|e| {
                crate::error::TitenError::ThreadsApiError(format!("Failed to delete post: {e}"))
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(crate::error::TitenError::ThreadsApiError(format!(
                "Delete post failed: HTTP {status} — {body}"
            )));
        }

        Ok(())
    }

    // ─── Insights ─────────────────────────────────────────────

    /// Fetch media insights using the official format.
    ///
    /// Official: `GET /v1.0/{threads_media_id}/insights?metric=likes,replies,...`
    ///
    /// Available metrics: `views`, `likes`, `replies`, `reposts`, `quotes`, `shares`
    ///
    /// Returns an array of metric objects, each with `name`, `period`, `values` (or
    /// `total_value`), `title`, `description`, `id`.
    pub async fn fetch_insights(
        &self,
        account: &crate::models::Account,
        threads_post_id: &str,
        metrics: Option<&str>,
    ) -> Result<Vec<InsightMetric>> {
        let fields = metrics.unwrap_or("likes,replies,reposts,quotes,views,shares");
        let url = format!(
            "{THREADS_GRAPH_API}/v1.0/{threads_post_id}/insights?metric={fields}&access_token={}",
            account.access_token
        );

        let resp = self.threads_get(&url).await?;

        let metrics = resp
            .get("data")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| serde_json::from_value(item.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();

        Ok(metrics)
    }

    /// Fetch user-level insights (profile views, follower count, etc.).
    ///
    /// Official: `GET /v1.0/{threads_user_id}/threads_insights?metric=...`
    ///
    /// Available metrics: `views`, `likes`, `replies`, `reposts`, `quotes`,
    /// `clicks`, `followers_count`, `follower_demographics`
    ///
    /// Optional `since`/`until` params (Unix timestamps). Default is 2-day range.
    pub async fn fetch_user_insights(
        &self,
        account: &crate::models::Account,
        metrics: &str,
        since: Option<i64>,
        until: Option<i64>,
    ) -> Result<Vec<UserInsightMetric>> {
        let mut url = format!(
            "{THREADS_GRAPH_API}/v1.0/{}/threads_insights?metric={}&access_token={}",
            account.user_id, metrics, account.access_token
        );

        if let Some(s) = since {
            url.push_str(&format!("&since={s}"));
        }
        if let Some(u) = until {
            url.push_str(&format!("&until={u}"));
        }

        let resp = self.threads_get(&url).await?;

        let insights = resp
            .get("data")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| serde_json::from_value(item.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();

        Ok(insights)
    }

    // ─── Replies ──────────────────────────────────────────────

    /// Fetch replies/comments for a post.
    ///
    /// Official: `GET /v1.0/{threads_post_id}/replies?fields=id,text,from,timestamp`
    pub async fn fetch_comments(
        &self,
        account: &crate::models::Account,
        threads_post_id: &str,
    ) -> Result<Vec<CommentData>> {
        let fields = "id,text,from,timestamp";
        let url = format!(
            "{THREADS_GRAPH_API}/v1.0/{threads_post_id}/replies?fields={fields}&access_token={}&limit=100",
            account.access_token
        );

        let resp = self.threads_get(&url).await?;

        let mut comments = Vec::new();

        if let Some(data) = resp.get("data").and_then(|d| d.as_array()) {
            for item in data {
                let from = item.get("from");
                comments.push(CommentData {
                    threads_comment_id: item
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    author_username: from
                        .and_then(|f| f.get("username"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    author_user_id: from
                        .and_then(|f| f.get("id"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    text: item
                        .get("text")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    timestamp: item
                        .get("timestamp")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                });
            }
        }

        Ok(comments)
    }

    /// Create a reply to a post (or to a specific reply).
    ///
    /// Official: `POST /v1.0/{threads_user_id}/threads` with `reply_to_id` param,
    /// then publish the container.
    ///
    /// This is a convenience method that combines both steps.
    pub async fn create_reply(
        &self,
        account: &crate::models::Account,
        reply_to: &str,
        text: &str,
    ) -> Result<String> {
        let params = ContainerParams {
            media_type: "TEXT".to_string(),
            text: Some(text.to_string()),
            image_url: None,
            video_url: None,
            topic_tag: None,
            link_attachment: None,
            gif_attachment: None,
            reply_to_id: Some(reply_to.to_string()),
            reply_control: None,
            is_carousel_item: None,
            enable_reply_approvals: None,
            children: None,
        };

        let container_id = self.create_container_full(account, &params).await?;
        let reply_id = self.publish_container(account, &container_id).await?;
        Ok(reply_id)
    }

    /// Hide or unhide a reply.
    ///
    /// Official: `POST /v1.0/{threads_reply_id}/manage_reply` with `hide=true|false`
    ///
    /// Automatically hides/unhides all nested replies under the top-level reply.
    pub async fn hide_reply(
        &self,
        account: &crate::models::Account,
        reply_id: &str,
        hide: bool,
    ) -> Result<bool> {
        let url = format!("{THREADS_GRAPH_API}/v1.0/{reply_id}/manage_reply");
        let body = serde_json::json!({
            "hide": hide,
            "access_token": account.access_token,
        });

        let resp = self.threads_post(&url, &body).await?;

        let success = resp
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok(success)
    }

    // ─── User Profile ──────────────────────────────────────────

    /// Fetch the authenticated user's own Threads profile.
    ///
    /// Official: `GET /v1.0/me?fields=id,username,name,...`
    ///
    /// Uses `/me` (app-scoped user) for the account owner.
    pub async fn fetch_my_profile(&self, account: &crate::models::Account) -> Result<UserProfile> {
        let fields = "id,username,name,threads_profile_picture_url,threads_biography,is_verified";
        let url = format!(
            "{THREADS_GRAPH_API}/v1.0/me?fields={fields}&access_token={}",
            account.access_token
        );

        let resp = self.threads_get(&url).await?;

        let profile: UserProfile = serde_json::from_value(resp.clone()).map_err(|e| {
            crate::error::TitenError::ThreadsApiError(format!("Failed to deserialize profile: {e}"))
        })?;

        Ok(profile)
    }

    /// Look up a public Threads profile by username.
    ///
    /// Official: `GET /v1.0/profile_lookup?username=...`
    ///
    /// Requires `threads_profile_discovery` permission.
    /// Limit: 1,000 requests per 24h. Only profiles with 100+ followers.
    pub async fn lookup_profile(
        &self,
        account: &crate::models::Account,
        username: &str,
    ) -> Result<serde_json::Value> {
        let url = format!(
            "{THREADS_GRAPH_API}/v1.0/profile_lookup?username={username}&access_token={}",
            account.access_token
        );

        let resp = self.threads_get(&url).await?;

        Ok(resp)
    }

    // ─── Publishing Limit ──────────────────────────────────────

    /// Fetch the user's current publishing quota.
    ///
    /// Official: `GET /v1.0/{threads_user_id}/threads_publishing_limit`
    ///
    /// Profiles are limited to 250 published posts within a 24-hour period.
    pub async fn fetch_publishing_limit(
        &self,
        account: &crate::models::Account,
    ) -> Result<PublishingLimit> {
        let url = format!(
            "{THREADS_GRAPH_API}/v1.0/{}/threads_publishing_limit?access_token={}",
            account.user_id, account.access_token
        );

        let resp = self.threads_get(&url).await?;

        // Threads API returns: { "data": [{ "quota_usage": N, "config": { "quota_total": N } }] }
        let entry = resp
            .get("data")
            .and_then(|d| d.as_array())
            .and_then(|arr| arr.first())
            .ok_or_else(|| {
                crate::error::TitenError::ThreadsApiError(
                    "Publishing limit response missing 'data' array".to_string(),
                )
            })?;

        let limit: PublishingLimit = serde_json::from_value(entry.clone()).map_err(|e| {
            crate::error::TitenError::ThreadsApiError(format!(
                "Failed to deserialize publishing limit: {e}"
            ))
        })?;

        Ok(limit)
    }

    // ─── Keyword Search ────────────────────────────────────────

    /// Search for public Threads posts by keyword.
    ///
    /// Official: `GET /v1.0/keyword_search?q=...&fields=...`
    ///
    /// Limit: 2,200 queries per 24h rolling period.
    /// Requires `threads_keyword_search` permission (otherwise only searches own posts).
    pub async fn search_keyword(
        &self,
        account: &crate::models::Account,
        query: &str,
        params: Option<&SearchParams>,
    ) -> Result<Vec<serde_json::Value>> {
        let search_fields = params.and_then(|p| p.fields.as_deref()).unwrap_or(
            "id,text,media_type,permalink,timestamp,username,has_replies,is_quote_post,is_reply",
        );
        let search_type = params
            .and_then(|p| p.search_type.as_deref())
            .unwrap_or("TOP");
        let limit_val = params.and_then(|p| p.limit).unwrap_or(25);

        let mut url = format!(
            "{THREADS_GRAPH_API}/v1.0/keyword_search?q={query}&search_type={search_type}&fields={search_fields}&limit={limit_val}&access_token={}",
            account.access_token
        );

        if let Some(Some(mode)) = params.map(|p| p.search_mode.as_deref()) {
            url.push_str(&format!("&search_mode={mode}"));
        }
        if let Some(Some(mt)) = params.map(|p| p.media_type.as_deref()) {
            url.push_str(&format!("&media_type={mt}"));
        }

        let resp = self.threads_get(&url).await?;

        let results = resp
            .get("data")
            .and_then(|d| d.as_array())
            .cloned()
            .unwrap_or_default();

        Ok(results)
    }

    // ─── Token Health Check ────────────────────────────────────

    /// Check token expiry for all accounts and refresh if needed.
    pub async fn check_all_tokens(&self) -> Vec<(String, String)> {
        let mut results = Vec::new();

        match self.store.list_accounts().await {
            Ok(accounts) => {
                for account in accounts {
                    if !account.is_active {
                        continue;
                    }
                    let status = account.token_status();
                    match status {
                        "expired" => {
                            warn!(
                                "Token expired for @{} — needs manual reauth",
                                account.username
                            );
                            results.push((account.username.clone(), "expired".to_string()));
                        }
                        "expiring_soon" => {
                            info!(
                                "Token expiring soon for @{} — refreshing...",
                                account.username
                            );
                            match self.refresh_token(&account).await {
                                Ok(updated) => {
                                    info!("Token refreshed for @{}", updated.username);
                                    results
                                        .push((account.username.clone(), "refreshed".to_string()));
                                }
                                Err(e) => {
                                    warn!("Token refresh failed for @{}: {e}", account.username);
                                    results.push((
                                        account.username.clone(),
                                        format!("refresh_failed: {e}"),
                                    ));
                                }
                            }
                        }
                        "valid" => {
                            results.push((account.username.clone(), "valid".to_string()));
                        }
                        _ => {
                            results.push((account.username.clone(), "unknown".to_string()));
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to list accounts for token check: {e}");
            }
        }

        results
    }
}

// ─── Container Parameters ──────────────────────────────────────

/// Full parameter set for creating a Threads media container.
///
/// Used by [`ThreadsClient::create_container_full`] to support all
/// official container creation options including carousels, topic tags,
/// link attachments, GIFs, reply control, and reply approvals.
#[derive(Debug, Clone, Default)]
pub struct ContainerParams {
    /// Required. `TEXT`, `IMAGE`, `VIDEO`, or `CAROUSEL`.
    pub media_type: String,
    /// Optional. Text content (required for TEXT type).
    pub text: Option<String>,
    /// Optional. Public URL to an image (required for IMAGE type).
    pub image_url: Option<String>,
    /// Optional. Public URL to a video (required for VIDEO type).
    pub video_url: Option<String>,
    /// Optional. Topic tag (1–50 chars, no `.` or `&`).
    pub topic_tag: Option<String>,
    /// Optional. Link preview URL (text-only posts, max 5 links total).
    pub link_attachment: Option<String>,
    /// Optional. GIF attachment `{"gif_id": "...", "provider": "GIPHY"}`.
    pub gif_attachment: Option<serde_json::Value>,
    /// Optional. ID of the post/reply to reply to.
    pub reply_to_id: Option<String>,
    /// Optional. Who can reply: `everyone`, `accounts_you_follow`,
    /// `mentioned_only`, `parent_post_author_only`, `followers_only`.
    pub reply_control: Option<String>,
    /// Optional. Mark as a carousel item child.
    pub is_carousel_item: Option<bool>,
    /// Optional. Enable reply approvals for this post.
    pub enable_reply_approvals: Option<bool>,
    /// Optional. Comma-separated child container IDs (for CAROUSEL type).
    pub children: Option<Vec<String>>,
}

// ─── Search Parameters ──────────────────────────────────────

/// Parameters for the Threads keyword/tag search endpoint.
///
/// Used by [`ThreadsClient::search_keyword`].
/// Limit: 2,200 queries per 24h rolling period.
#[derive(Debug, Clone, Default)]
pub struct SearchParams {
    /// Comma-separated fields to return.
    pub fields: Option<String>,
    /// `TOP` (default) or `RECENT`.
    pub search_type: Option<String>,
    /// `KEYWORD` (default) or `TAG`.
    pub search_mode: Option<String>,
    /// Filter by media type: `TEXT`, `IMAGE`, `VIDEO`.
    pub media_type: Option<String>,
    /// Max results per page (default 25, max 100).
    pub limit: Option<u32>,
}
