use crate::server::AppState;
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
