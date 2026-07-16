use crate::error::Result;
use crate::models::Account;
use crate::store::Store;
use reqwest::Client;
use tracing::{info, warn};

const THREADS_GRAPH_API: &str = "https://graph.threads.net";

/// Threads API client for interacting with the Threads Graph API
pub struct ThreadsClient {
    http: Client,
    store: std::sync::Arc<Store>,
}

impl ThreadsClient {
    pub fn new(store: std::sync::Arc<Store>) -> Self {
        Self {
            http: Client::new(),
            store,
        }
    }

    /// Refresh an account's access token via the Threads API
    ///
    /// `GET /refresh_access_token?grant_type=th_refresh_token&access_token={token}`
    pub async fn refresh_token(&self, account: &Account) -> Result<Account> {
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
            refresh_token: None,
            expires_at: Some(expires_at_str),
            is_active: None,
        };

        let account = self.store.update_account(&account.id, &update).await?;
        Ok(account)
    }

    /// Create a Threads container (first step for media posts)
    ///
    /// Two-step flow: create container → publish
    pub async fn create_container(
        &self,
        account: &Account,
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

        let url = format!("{THREADS_GRAPH_API}/v1.0/me/threads");
        let resp: serde_json::Value = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await?
            .json()
            .await
            .map_err(|e| {
                crate::error::TitenError::ThreadsApiError(format!(
                    "Failed to parse container response: {e}"
                ))
            })?;

        let container_id = resp.get("id").and_then(|v| v.as_str()).ok_or_else(|| {
            crate::error::TitenError::ThreadsApiError("No container id in response".to_string())
        })?;

        Ok(container_id.to_string())
    }

    /// Publish a Threads container (second step)
    pub async fn publish_container(&self, account: &Account, creation_id: &str) -> Result<String> {
        let url = format!("{THREADS_GRAPH_API}/v1.0/me/threads_publish");
        let body = serde_json::json!({
            "access_token": account.access_token,
            "creation_id": creation_id,
        });

        let resp: serde_json::Value = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await?
            .json()
            .await
            .map_err(|e| {
                crate::error::TitenError::ThreadsApiError(format!(
                    "Failed to parse publish response: {e}"
                ))
            })?;

        let post_id = resp.get("id").and_then(|v| v.as_str()).ok_or_else(|| {
            crate::error::TitenError::ThreadsApiError("No post id in publish response".to_string())
        })?;

        Ok(post_id.to_string())
    }

    /// Create and publish a text post in a single step (one-step publishing)
    pub async fn publish_text(&self, account: &Account, caption: &str) -> Result<String> {
        // Create container
        let container_id = self
            .create_container(account, "TEXT", Some(caption), None, None)
            .await?;

        // Publish immediately
        let post_id = self.publish_container(account, &container_id).await?;
        Ok(post_id)
    }

    /// Create and publish an image post (two-step)
    pub async fn publish_image(
        &self,
        account: &Account,
        caption: Option<&str>,
        image_url: &str,
        alt_text: Option<&str>,
    ) -> Result<String> {
        let _alt_text = alt_text;
        let container_id = self
            .create_container(account, "IMAGE", caption, Some(image_url), None)
            .await?;

        let post_id = self.publish_container(account, &container_id).await?;
        Ok(post_id)
    }

    /// Delete a post on Threads
    pub async fn delete_post(&self, account: &Account, threads_post_id: &str) -> Result<()> {
        let url = format!("{THREADS_GRAPH_API}/v1.0/{threads_post_id}");
        let body = serde_json::json!({
            "access_token": account.access_token,
        });

        self.http
            .delete(&url)
            .query(&body)
            .send()
            .await
            .map_err(|e| {
                crate::error::TitenError::ThreadsApiError(format!("Failed to delete post: {e}"))
            })?;

        Ok(())
    }

    /// Fetch post insights (likes, replies, reposts, views, quotes)
    pub async fn fetch_insights(
        &self,
        account: &Account,
        threads_post_id: &str,
    ) -> Result<crate::models::Insights> {
        let fields = "like_count,replies,root_repost_count,root_quote_count,views";
        let url = format!(
            "{THREADS_GRAPH_API}/v1.0/{threads_post_id}?fields={fields}&access_token={}",
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
                        "Failed to parse insights response: {e}"
                    ))
                })?;

        Ok(crate::models::Insights {
            likes: resp.pointer("/like_count").and_then(|v| v.as_i64()),
            replies: resp.pointer("/replies").and_then(|v| v.as_i64()),
            reposts: resp.pointer("/root_repost_count").and_then(|v| v.as_i64()),
            views: resp.pointer("/views").and_then(|v| v.as_i64()),
            quotes: resp.pointer("/root_quote_count").and_then(|v| v.as_i64()),
        })
    }

    /// Fetch replies/comments for a post
    pub async fn fetch_comments(
        &self,
        account: &Account,
        threads_post_id: &str,
    ) -> Result<Vec<crate::models::CommentData>> {
        let fields = "id,text,from,timestamp";
        let url = format!(
            "{THREADS_GRAPH_API}/v1.0/{threads_post_id}/replies?fields={fields}&access_token={}&limit=100",
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
                        "Failed to parse comments response: {e}"
                    ))
                })?;

        let mut comments = Vec::new();

        if let Some(data) = resp.get("data").and_then(|d| d.as_array()) {
            for item in data {
                let from = item.get("from");
                comments.push(crate::models::CommentData {
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

    /// Check token expiry for all accounts and refresh if needed
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
