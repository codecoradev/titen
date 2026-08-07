use crate::server::AppState;
use axum::http::StatusCode;
use axum::{
    Json,
    extract::{Path, State},
};

/// Fetch the Threads user profile for an account
pub async fn get_user_profile(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let account = match state.store.get_account(&id).await {
        Ok(a) => a,
        Err(e) => return Json(serde_json::json!({ "error": e.to_string(), "code": "NOT_FOUND" })),
    };

    match state.threads_client.fetch_my_profile(&account).await {
        Ok(profile) => Json(serde_json::json!({ "data": profile })),
        Err(e) => {
            Json(serde_json::json!({ "error": e.to_string(), "code": "PROFILE_FETCH_FAILED" }))
        }
    }
}

/// Fetch the Threads publishing quota/limit for an account
pub async fn get_publishing_limit(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let account = match state.store.get_account(&id).await {
        Ok(a) => a,
        Err(e) => return Json(serde_json::json!({ "error": e.to_string(), "code": "NOT_FOUND" })),
    };

    match state.threads_client.fetch_publishing_limit(&account).await {
        Ok(limits) => Json(serde_json::json!({ "data": limits })),
        Err(e) => {
            Json(serde_json::json!({ "error": e.to_string(), "code": "LIMITS_FETCH_FAILED" }))
        }
    }
}

/// Check and refresh all account tokens
pub async fn check_all_tokens(State(state): State<AppState>) -> Json<serde_json::Value> {
    let results = state.threads_client.check_all_tokens().await;
    let data: Vec<serde_json::Value> = results
        .into_iter()
        .map(|(username, status)| {
            serde_json::json!({
                "username": username,
                "status": status,
            })
        })
        .collect();
    Json(serde_json::json!({ "data": data }))
}

/// Create a Threads container (first step for media posts / carousel)
pub async fn create_container(
    State(state): State<AppState>,
    Json(input): Json<CreateContainerInput>,
) -> Json<serde_json::Value> {
    let account = match state.store.get_account(&input.account_id).await {
        Ok(a) => a,
        Err(e) => {
            return Json(
                serde_json::json!({ "error": e.to_string(), "code": "ACCOUNT_NOT_FOUND" }),
            );
        }
    };

    match state
        .threads_client
        .create_container(
            &account,
            &input.media_type,
            input.text.as_deref(),
            input.image_url.as_deref(),
            input.video_url.as_deref(),
        )
        .await
    {
        Ok(container_id) => Json(serde_json::json!({
            "data": {
                "container_id": container_id,
                "media_type": input.media_type,
                "status": "created",
            }
        })),
        Err(e) => {
            Json(serde_json::json!({ "error": e.to_string(), "code": "CONTAINER_CREATE_FAILED" }))
        }
    }
}

/// Publish a previously created Threads container
pub async fn publish_container(
    State(state): State<AppState>,
    Path(container_id): Path<String>,
    Json(input): Json<PublishContainerInput>,
) -> Json<serde_json::Value> {
    let account = match state.store.get_account(&input.account_id).await {
        Ok(a) => a,
        Err(e) => {
            return Json(
                serde_json::json!({ "error": e.to_string(), "code": "ACCOUNT_NOT_FOUND" }),
            );
        }
    };

    match state
        .threads_client
        .publish_container(&account, &container_id)
        .await
    {
        Ok(post_id) => Json(serde_json::json!({
            "data": {
                "post_id": post_id,
                "container_id": container_id,
                "status": "published",
            }
        })),
        Err(e) => {
            Json(serde_json::json!({ "error": e.to_string(), "code": "CONTAINER_PUBLISH_FAILED" }))
        }
    }
}

/// Check the status of a container (e.g., for media processing completion)
pub async fn get_container_status(
    State(state): State<AppState>,
    Path(container_id): Path<String>,
    Json(input): Json<ContainerStatusInput>,
) -> Json<serde_json::Value> {
    let account = match state.store.get_account(&input.account_id).await {
        Ok(a) => a,
        Err(e) => {
            return Json(
                serde_json::json!({ "error": e.to_string(), "code": "ACCOUNT_NOT_FOUND" }),
            );
        }
    };

    match state
        .threads_client
        .check_container_status(&account, &container_id)
        .await
    {
        Ok(status) => Json(serde_json::json!({ "data": status })),
        Err(e) => {
            Json(serde_json::json!({ "error": e.to_string(), "code": "CONTAINER_STATUS_FAILED" }))
        }
    }
}

use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateContainerInput {
    pub account_id: String,
    pub media_type: String,
    pub text: Option<String>,
    pub image_url: Option<String>,
    pub video_url: Option<String>,
}

#[derive(Deserialize)]
pub struct PublishContainerInput {
    pub account_id: String,
}

#[derive(Deserialize)]
pub struct ContainerStatusInput {
    pub account_id: String,
}

#[derive(Deserialize)]
pub struct CreateReplyInput {
    pub account_id: String,
    pub reply_to: String,
    pub text: String,
}

#[derive(Deserialize)]
pub struct HideReplyInput {
    pub account_id: String,
    pub hide: bool,
}

#[derive(Deserialize)]
pub struct LookupProfileInput {
    pub account_id: String,
    pub username: String,
}

#[derive(Deserialize)]
pub struct SearchKeywordInput {
    pub account_id: String,
    pub query: String,
    pub search_type: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Deserialize)]
pub struct FetchMentionsInput {
    pub account_id: String,
    pub limit: Option<u32>,
}

#[derive(Deserialize)]
pub struct ShareToInstagramInput {
    pub account_id: String,
    pub threads_post_id: String,
}

/// Create a reply to a Threads post
pub async fn create_reply(
    State(state): State<AppState>,
    Json(input): Json<CreateReplyInput>,
) -> Json<serde_json::Value> {
    let account = match state.store.get_account(&input.account_id).await {
        Ok(a) => a,
        Err(e) => {
            return Json(
                serde_json::json!({ "error": e.to_string(), "code": "ACCOUNT_NOT_FOUND" }),
            );
        }
    };

    match state
        .threads_client
        .create_reply(&account, &input.reply_to, &input.text)
        .await
    {
        Ok(reply_id) => {
            Json(serde_json::json!({ "data": { "reply_id": reply_id, "status": "published" } }))
        }
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "REPLY_FAILED" })),
    }
}

/// Hide or unhide a reply
pub async fn hide_reply(
    State(state): State<AppState>,
    Path(reply_id): Path<String>,
    Json(input): Json<HideReplyInput>,
) -> Json<serde_json::Value> {
    let account = match state.store.get_account(&input.account_id).await {
        Ok(a) => a,
        Err(e) => {
            return Json(
                serde_json::json!({ "error": e.to_string(), "code": "ACCOUNT_NOT_FOUND" }),
            );
        }
    };

    match state
        .threads_client
        .hide_reply(&account, &reply_id, input.hide)
        .await
    {
        Ok(success) => {
            Json(serde_json::json!({ "data": { "reply_id": reply_id, "hidden": success } }))
        }
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "HIDE_REPLY_FAILED" })),
    }
}

/// Look up a public Threads profile by username
pub async fn lookup_profile(
    State(state): State<AppState>,
    Json(input): Json<LookupProfileInput>,
) -> Json<serde_json::Value> {
    let account = match state.store.get_account(&input.account_id).await {
        Ok(a) => a,
        Err(e) => {
            return Json(
                serde_json::json!({ "error": e.to_string(), "code": "ACCOUNT_NOT_FOUND" }),
            );
        }
    };

    match state
        .threads_client
        .lookup_profile(&account, &input.username)
        .await
    {
        Ok(profile) => Json(serde_json::json!({ "data": profile })),
        Err(e) => {
            Json(serde_json::json!({ "error": e.to_string(), "code": "PROFILE_LOOKUP_FAILED" }))
        }
    }
}

/// Search for public Threads posts by keyword
pub async fn search_keyword(
    State(state): State<AppState>,
    Json(input): Json<SearchKeywordInput>,
) -> Json<serde_json::Value> {
    let account = match state.store.get_account(&input.account_id).await {
        Ok(a) => a,
        Err(e) => {
            return Json(
                serde_json::json!({ "error": e.to_string(), "code": "ACCOUNT_NOT_FOUND" }),
            );
        }
    };

    let params = titen_core::threads_client::SearchParams {
        search_type: input.search_type,
        limit: input.limit,
        ..Default::default()
    };

    match state
        .threads_client
        .search_keyword(&account, &input.query, Some(&params))
        .await
    {
        Ok(results) => Json(serde_json::json!({ "data": results, "count": results.len() })),
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "SEARCH_FAILED" })),
    }
}

/// Fetch account-level insights (aggregate metrics across all posts).
///
/// GET /api/accounts/{id}/insights?metrics=views,likes,replies,reposts,quotes&since=...&until=...
pub async fn get_account_insights(
    State(state): State<AppState>,
    Path(id): Path<String>,
    axum::extract::Query(params): axum::extract::Query<AccountInsightsQuery>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<serde_json::Value>)> {
    let account = match state.store.get_account(&id).await {
        Ok(a) => a,
        Err(e) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e.to_string(), "code": "NOT_FOUND" })),
            ));
        }
    };

    let metrics = params
        .metrics
        .as_deref()
        .unwrap_or("views,likes,replies,reposts,quotes,followers_count");

    match state
        .threads_client
        .fetch_user_insights(&account, metrics, params.since, params.until)
        .await
    {
        Ok(insights) => Ok((
            StatusCode::OK,
            Json(serde_json::json!({ "data": insights })),
        )),
        Err(e) => Err((
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": e.to_string(), "code": "INSIGHTS_FETCH_FAILED" })),
        )),
    }
}

/// Query parameters for account insights
#[derive(serde::Deserialize)]
pub struct AccountInsightsQuery {
    pub metrics: Option<String>,
    pub since: Option<i64>,
    pub until: Option<i64>,
}

// ─── Mentions ──────────────────────────────────────────────

/// Fetch posts where the user is mentioned
pub async fn fetch_mentions(
    State(state): State<AppState>,
    Json(input): Json<FetchMentionsInput>,
) -> Json<serde_json::Value> {
    let account = match state.store.get_account(&input.account_id).await {
        Ok(a) => a,
        Err(e) => {
            return Json(
                serde_json::json!({ "error": e.to_string(), "code": "ACCOUNT_NOT_FOUND" }),
            );
        }
    };

    match state
        .threads_client
        .fetch_mentions(&account, input.limit)
        .await
    {
        Ok(mentions) => Json(serde_json::json!({ "data": mentions, "count": mentions.len() })),
        Err(e) => {
            Json(serde_json::json!({ "error": e.to_string(), "code": "MENTIONS_FETCH_FAILED" }))
        }
    }
}

/// Crosspost a published Threads post to Instagram
pub async fn share_to_instagram(
    State(state): State<AppState>,
    Json(input): Json<ShareToInstagramInput>,
) -> Json<serde_json::Value> {
    let account = match state.store.get_account(&input.account_id).await {
        Ok(a) => a,
        Err(e) => {
            return Json(
                serde_json::json!({ "error": e.to_string(), "code": "ACCOUNT_NOT_FOUND" }),
            );
        }
    };

    match state
        .threads_client
        .share_to_instagram(&account, &input.threads_post_id)
        .await
    {
        Ok(success) => Json(
            serde_json::json!({ "data": { "threads_post_id": input.threads_post_id, "shared": success } }),
        ),
        Err(e) => Json(serde_json::json!({ "error": e.to_string(), "code": "SHARE_FAILED" })),
    }
}
