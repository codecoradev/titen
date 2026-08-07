//! Titen MCP server — stdio JSON-RPC 2.0 interface for AI agents.
//!
//! Implements the [Model Context Protocol](https://modelcontextprotocol.io/) over stdio,
//! exposing Titen operations as tools that Claude Desktop, Cursor, and other MCP
//! clients can call directly.
//!
//! # Available tools
//!
//! - `list_accounts` — list all managed Threads accounts
//! - `get_user_profile` — fetch a Threads user profile from the API
//! - `get_publishing_limit` — fetch account's Threads publishing quota
//! - `create_post` — create and publish a post
//! - `schedule_post` — schedule a post for future publishing
//! - `list_schedules` — list scheduled posts
//! - `cancel_schedule` — cancel a scheduled post
//! - `refresh_token` — refresh an account's access token
//! - `check_tokens` — check and auto-refresh all accounts' tokens
//! - `fetch_comments` — fetch and store comments from a Threads post
//! - `get_post_sentiment` — get sentiment analysis for a post's comments
//! - `get_post_insights` — fetch post insights from Threads API
//! - `get_account_analytics` — get analytics summary for an account
//! - `delete_post` — delete a post
//! - `create_container` — create a Threads container (for carousel/media posts)
//! - `publish_container` — publish a previously created container
//!
//! # Configuration
//!
//! Set `TITEN_DB_PATH` to point to the SQLite database. Defaults to `~/.codecoradev/titen/titen.db`.

use std::io::{self, BufRead, Write};
use std::sync::Arc;

use serde_json::json;
use titen_core::{Store, ThreadsClient};

fn main() {
    eprintln!("titen-mcp starting (stdio JSON-RPC)");

    // Initialize tokio runtime for async store operations
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to build tokio runtime");

    // Initialize store (blocking)
    let db_path = titen_core::config::default_db_path();
    titen_core::config::ensure_parent_dir(&db_path);
    let (store, threads_client) = rt.block_on(async {
        let pool = sqlx::SqlitePool::connect(&format!("sqlite:{db_path}?mode=rwc"))
            .await
            .expect("Failed to connect to database");
        let store = Store::new(pool.clone());
        store.migrate().await.expect("Failed to run migrations");
        let store = Arc::new(store);
        let threads_client = Arc::new(ThreadsClient::new(store.clone()));
        (store, threads_client)
    });

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let request: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                let resp = json_rpc_error(0, &format!("Parse error: {e}"));
                let _ = writeln!(
                    stdout,
                    "{}",
                    serde_json::to_string(&resp).unwrap_or_default()
                );
                let _ = stdout.flush();
                continue;
            }
        };

        let id = request.get("id").cloned().unwrap_or(json!(null));
        let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");

        let result = match method {
            "initialize" => {
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": {} },
                    "serverInfo": {
                        "name": "titen",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                })
            }
            "notifications/initialized" => {
                // Ack notification, no response needed
                json!(null)
            }
            "tools/list" => tools_list(),
            "tools/call" => {
                let tool_name = request
                    .pointer("/params/name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let arguments = request
                    .pointer("/params/arguments")
                    .cloned()
                    .unwrap_or(json!({}));
                handle_tool_call(&rt, &store, &threads_client, tool_name, arguments)
            }
            _ => json_rpc_error_value(&id, "Method not found", -32601),
        };

        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        });

        let _ = writeln!(
            stdout,
            "{}",
            serde_json::to_string(&response).unwrap_or_default()
        );
        let _ = stdout.flush();
    }
}

fn tools_list() -> serde_json::Value {
    json!({
        "tools": [
            {
                "name": "list_accounts",
                "description": "List all Threads accounts managed by titen",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "get_user_profile",
                "description": "Fetch a Threads user's profile from the Threads API",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "account_id": { "type": "string", "description": "Account ID" }
                    },
                    "required": ["account_id"]
                }
            },
            {
                "name": "get_publishing_limit",
                "description": "Fetch an account's Threads publishing quota (daily post limit, etc.)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "account_id": { "type": "string", "description": "Account ID" }
                    },
                    "required": ["account_id"]
                }
            },
            {
                "name": "create_post",
                "description": "Create and publish a Threads post",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "account_id": { "type": "string", "description": "Account ID to post from" },
                        "caption": { "type": "string", "description": "Post caption text" },
                        "media_type": { "type": "string", "enum": ["TEXT", "IMAGE"], "description": "Media type" },
                        "image_url": { "type": "string", "description": "Image URL for IMAGE posts" },
                        "alt_text": { "type": "string", "description": "Alt text for image accessibility" }
                    },
                    "required": ["account_id", "caption"]
                }
            },
            {
                "name": "schedule_post",
                "description": "Schedule a post for future publishing",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "account_id": { "type": "string", "description": "Account ID" },
                        "caption": { "type": "string", "description": "Post caption" },
                        "scheduled_at": { "type": "string", "description": "ISO 8601 datetime" },
                        "media_type": { "type": "string", "description": "Media type (TEXT/IMAGE)" }
                    },
                    "required": ["account_id", "caption", "scheduled_at"]
                }
            },
            {
                "name": "list_schedules",
                "description": "List scheduled posts",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "account_id": { "type": "string" },
                        "status": { "type": "string", "enum": ["pending", "published", "failed", "cancelled"] }
                    }
                }
            },
            {
                "name": "cancel_schedule",
                "description": "Cancel a scheduled post",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Schedule ID" }
                    },
                    "required": ["id"]
                }
            },
            {
                "name": "refresh_token",
                "description": "Refresh an account's Threads access token",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "account_id": { "type": "string", "description": "Account ID to refresh" }
                    },
                    "required": ["account_id"]
                }
            },
            {
                "name": "check_tokens",
                "description": "Check all accounts' token expiry status and auto-refresh expiring tokens",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "fetch_comments",
                "description": "Fetch and store comments from a Threads post via the Threads API",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "post_id": { "type": "string", "description": "Post ID" }
                    },
                    "required": ["post_id"]
                }
            },
            {
                "name": "get_post_sentiment",
                "description": "Get sentiment analysis for a post's comments",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "post_id": { "type": "string", "description": "Post ID" }
                    },
                    "required": ["post_id"]
                }
            },
            {
                "name": "get_post_insights",
                "description": "Fetch post insights (likes, replies, reposts, views, quotes) from the Threads API",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "post_id": { "type": "string", "description": "Post ID" }
                    },
                    "required": ["post_id"]
                }
            },
            {
                "name": "get_account_analytics",
                "description": "Get analytics summary for an account's posts",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "account_id": { "type": "string", "description": "Account ID" }
                    },
                    "required": ["account_id"]
                }
            },
            {
                "name": "delete_post",
                "description": "Delete a post from Threads and the local database",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "post_id": { "type": "string", "description": "Post ID to delete" }
                    },
                    "required": ["post_id"]
                }
            },
            {
                "name": "create_container",
                "description": "Create a Threads container (first step for media posts, carousel, etc.)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "account_id": { "type": "string", "description": "Account ID" },
                        "media_type": { "type": "string", "description": "Media type (TEXT, IMAGE, VIDEO)" },
                        "text": { "type": "string", "description": "Caption text" },
                        "image_url": { "type": "string", "description": "Image URL for IMAGE posts" },
                        "video_url": { "type": "string", "description": "Video URL for VIDEO posts" }
                    },
                    "required": ["account_id", "media_type"]
                }
            },
            {
                "name": "publish_container",
                "description": "Publish a previously created Threads container by container ID",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "account_id": { "type": "string", "description": "Account ID" },
                        "container_id": { "type": "string", "description": "Container ID from create_container" }
                    },
                    "required": ["account_id", "container_id"]
                }
            }
        ]
    })
}

fn handle_tool_call(
    rt: &tokio::runtime::Runtime,
    store: &Arc<Store>,
    threads_client: &Arc<ThreadsClient>,
    tool_name: &str,
    args: serde_json::Value,
) -> serde_json::Value {
    let result = match tool_name {
        "list_accounts" => rt.block_on(async {
            match store.list_accounts().await {
                Ok(accounts) => {
                    let data: Vec<serde_json::Value> = accounts
                        .into_iter()
                        .map(|a| {
                            json!({
                                "id": a.id,
                                "username": a.username,
                                "is_active": a.is_active,
                                "token_status": a.token_status(),
                            })
                        })
                        .collect();
                    Ok(json!(data))
                }
                Err(e) => Err(format!("Failed to list accounts: {e}")),
            }
        }),
        "get_user_profile" => rt.block_on(async {
            let account_id = args
                .get("account_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            match store.get_account(account_id).await {
                Ok(account) => match threads_client.fetch_my_profile(&account).await {
                    Ok(profile) => Ok(json!(profile)),
                    Err(e) => Err(format!("Failed to fetch user profile: {e}")),
                },
                Err(e) => Err(format!("Account not found: {e}")),
            }
        }),
        "get_publishing_limit" => rt.block_on(async {
            let account_id = args
                .get("account_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            match store.get_account(account_id).await {
                Ok(account) => match threads_client.fetch_publishing_limit(&account).await {
                    Ok(limits) => Ok(json!(limits)),
                    Err(e) => Err(format!("Failed to fetch publishing limit: {e}")),
                },
                Err(e) => Err(format!("Account not found: {e}")),
            }
        }),
        "create_post" => rt.block_on(async {
            let account_id = args
                .get("account_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let caption = args.get("caption").and_then(|v| v.as_str()).unwrap_or("");
            let media_type = args
                .get("media_type")
                .and_then(|v| v.as_str())
                .unwrap_or("TEXT");
            let image_url = args.get("image_url").and_then(|v| v.as_str());

            let input = titen_core::models::CreatePost {
                account_id: account_id.to_string(),
                caption: Some(caption.to_string()),
                media_type: Some(media_type.to_string()),
                image_url: image_url.map(|s| s.to_string()),
                text_attachment: None,
                video_url: None,
                image_urls: None,
                media_ids: None,
                alt_text: args
                    .get("alt_text")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            };

            let id = uuid::Uuid::now_v7().to_string();
            match store.create_post(&id, &input).await {
                Ok(post) => Ok(json!({
                    "id": post.id,
                    "status": post.status,
                    "caption": post.caption,
                })),
                Err(e) => Err(format!("Failed to create post: {e}")),
            }
        }),
        "schedule_post" => rt.block_on(async {
            let account_id = args
                .get("account_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let caption = args.get("caption").and_then(|v| v.as_str()).unwrap_or("");
            let scheduled_at = args
                .get("scheduled_at")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let media_type = args
                .get("media_type")
                .and_then(|v| v.as_str())
                .unwrap_or("TEXT");

            let input = titen_core::models::CreateSchedule {
                account_id: account_id.to_string(),
                caption: Some(caption.to_string()),
                media_type: Some(media_type.to_string()),
                text_attachment: None,
                media_urls: args
                    .get("image_url")
                    .and_then(|v| v.as_str())
                    .map(|s| vec![s.to_string()]),
                scheduled_at: scheduled_at.to_string(),
            };

            let id = uuid::Uuid::now_v7().to_string();
            match store.create_schedule(&id, &input).await {
                Ok(schedule) => Ok(json!({
                    "id": schedule.id,
                    "scheduled_at": schedule.scheduled_at,
                    "status": schedule.status,
                })),
                Err(e) => Err(format!("Failed to create schedule: {e}")),
            }
        }),
        "list_schedules" => rt.block_on(async {
            let account_id = args.get("account_id").and_then(|v| v.as_str());
            let status = args.get("status").and_then(|v| v.as_str());
            match store.list_schedules(account_id, status).await {
                Ok(schedules) => {
                    let data: Vec<serde_json::Value> = schedules
                        .into_iter()
                        .map(|s| {
                            json!({
                                "id": s.id,
                                "account_id": s.account_id,
                                "caption": s.caption,
                                "scheduled_at": s.scheduled_at,
                                "status": s.status,
                            })
                        })
                        .collect();
                    Ok(json!(data))
                }
                Err(e) => Err(format!("Failed to list schedules: {e}")),
            }
        }),
        "cancel_schedule" => rt.block_on(async {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            match store.delete_schedule(id).await {
                Ok(()) => Ok(json!({ "deleted": id })),
                Err(e) => Err(format!("Failed to cancel schedule: {e}")),
            }
        }),
        "refresh_token" => rt.block_on(async {
            let account_id = args
                .get("account_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            match store.get_account(account_id).await {
                Ok(account) => match threads_client.refresh_token(&account).await {
                    Ok(updated) => Ok(json!({
                        "id": updated.id,
                        "username": updated.username,
                        "token_status": updated.token_status(),
                        "expires_at": updated.expires_at,
                    })),
                    Err(e) => Err(format!("Failed to refresh token: {e}")),
                },
                Err(e) => Err(format!("Account not found: {e}")),
            }
        }),
        "check_tokens" => rt.block_on(async {
            let results = threads_client.check_all_tokens().await;
            let data: Vec<serde_json::Value> = results
                .into_iter()
                .map(|(username, status)| {
                    json!({
                        "username": username,
                        "status": status,
                    })
                })
                .collect();
            Ok(json!(data))
        }),
        "fetch_comments" => rt.block_on(async {
            let post_id = args.get("post_id").and_then(|v| v.as_str()).unwrap_or("");

            // Get post + account for live fetch
            let post = match store.get_post(post_id).await {
                Ok(p) => p,
                Err(e) => return Err(format!("Post not found: {e}")),
            };

            let threads_post_id = match &post.threads_post_id {
                Some(id) => id.clone(),
                None => return Err("Post not yet published to Threads".to_string()),
            };

            let account = match store.get_account(&post.account_id).await {
                Ok(a) => a,
                Err(e) => return Err(format!("Account not found: {e}")),
            };

            // Fetch from Threads API
            let comment_data = match threads_client
                .fetch_comments(&account, &threads_post_id)
                .await
            {
                Ok(data) => data,
                Err(e) => return Err(format!("Failed to fetch comments: {e}")),
            };

            // Store in DB
            let mut stored = Vec::new();
            for cd in &comment_data {
                let id = uuid::Uuid::now_v7().to_string();
                match store
                    .insert_comment(
                        &id,
                        post_id,
                        cd.author_username.as_deref(),
                        cd.author_user_id.as_deref(),
                        &cd.text,
                    )
                    .await
                {
                    Ok(c) => stored.push(c),
                    Err(_) => continue, // skip duplicates
                }
            }

            Ok(json!({
                "comments": stored,
                "fetched": comment_data.len(),
                "stored": stored.len(),
            }))
        }),
        "get_post_sentiment" => rt.block_on(async {
            let post_id = args.get("post_id").and_then(|v| v.as_str()).unwrap_or("");
            let comments = match store.list_comments(post_id).await {
                Ok(c) => c,
                Err(e) => return Err(format!("Failed to list comments: {e}")),
            };

            let total = comments.len();
            let analyzed: Vec<_> = comments.iter().filter(|c| c.sentiment.is_some()).collect();
            let positive = analyzed
                .iter()
                .filter(|c| c.sentiment.as_deref() == Some("positive"))
                .count();
            let negative = analyzed
                .iter()
                .filter(|c| c.sentiment.as_deref() == Some("negative"))
                .count();
            let neutral = analyzed
                .iter()
                .filter(|c| c.sentiment.as_deref() == Some("neutral"))
                .count();

            Ok(json!({
                "total": total,
                "analyzed": analyzed.len(),
                "positive": positive,
                "negative": negative,
                "neutral": neutral,
            }))
        }),
        "get_post_insights" => rt.block_on(async {
            let post_id = args.get("post_id").and_then(|v| v.as_str()).unwrap_or("");

            let post = match store.get_post(post_id).await {
                Ok(p) => p,
                Err(e) => return Err(format!("Post not found: {e}")),
            };

            let threads_post_id = match &post.threads_post_id {
                Some(id) => id.clone(),
                None => return Err("Post not yet published to Threads".to_string()),
            };

            let account = match store.get_account(&post.account_id).await {
                Ok(a) => a,
                Err(e) => return Err(format!("Account not found: {e}")),
            };

            match threads_client
                .fetch_insights(&account, &threads_post_id, None)
                .await
            {
                Ok(insights) => {
                    // Store snapshot
                    let snap_id = uuid::Uuid::now_v7().to_string();
                    let insights_model: titen_core::models::Insights = insights.into();
                    let _ = store
                        .insert_analytics_snap(&snap_id, post_id, &insights_model)
                        .await;
                    Ok(json!(insights_model))
                }
                Err(e) => Err(format!("Failed to fetch insights: {e}")),
            }
        }),
        "get_account_analytics" => rt.block_on(async {
            let account_id = args
                .get("account_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            match store
                .list_posts(Some(account_id), Some("published"), 100, 0)
                .await
            {
                Ok(posts) => {
                    let data: Vec<serde_json::Value> = posts
                        .into_iter()
                        .map(|p| {
                            json!({
                                "id": p.id,
                                "caption": p.caption,
                                "status": p.status,
                            })
                        })
                        .collect();
                    Ok(json!({ "posts": data, "count": data.len() }))
                }
                Err(e) => Err(format!("Failed to get analytics: {e}")),
            }
        }),
        "delete_post" => rt.block_on(async {
            let post_id = args.get("post_id").and_then(|v| v.as_str()).unwrap_or("");

            // Try to delete from Threads API first
            if let Ok(post) = store.get_post(post_id).await {
                if let Some(threads_post_id) = &post.threads_post_id {
                    if let Ok(account) = store.get_account(&post.account_id).await {
                        let _ = threads_client.delete_post(&account, threads_post_id).await;
                    }
                }
            }

            match store.delete_post(post_id).await {
                Ok(()) => Ok(json!({ "deleted": post_id })),
                Err(e) => Err(format!("Failed to delete post: {e}")),
            }
        }),
        "create_container" => rt.block_on(async {
            let account_id = args
                .get("account_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let media_type = args
                .get("media_type")
                .and_then(|v| v.as_str())
                .unwrap_or("TEXT");
            let text = args.get("text").and_then(|v| v.as_str());
            let image_url = args.get("image_url").and_then(|v| v.as_str());
            let video_url = args.get("video_url").and_then(|v| v.as_str());

            match store.get_account(account_id).await {
                Ok(account) => {
                    match threads_client
                        .create_container(&account, media_type, text, image_url, video_url)
                        .await
                    {
                        Ok(container_id) => Ok(json!({
                            "container_id": container_id,
                            "media_type": media_type,
                        })),
                        Err(e) => Err(format!("Failed to create container: {e}")),
                    }
                }
                Err(e) => Err(format!("Account not found: {e}")),
            }
        }),
        "publish_container" => rt.block_on(async {
            let account_id = args
                .get("account_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let container_id = args
                .get("container_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            match store.get_account(account_id).await {
                Ok(account) => {
                    match threads_client
                        .publish_container(&account, container_id)
                        .await
                    {
                        Ok(post_id) => Ok(json!({
                            "post_id": post_id,
                            "container_id": container_id,
                        })),
                        Err(e) => Err(format!("Failed to publish container: {e}")),
                    }
                }
                Err(e) => Err(format!("Account not found: {e}")),
            }
        }),
        _ => Err(format!("Unknown tool: {tool_name}")),
    };

    match result {
        Ok(data) => json!({
            "content": [{ "type": "text", "text": serde_json::to_string_pretty(&data).unwrap_or_else(|_| data.to_string()) }]
        }),
        Err(msg) => json!({
            "content": [{ "type": "text", "text": format!("Error: {msg}") }],
            "isError": true
        }),
    }
}

fn json_rpc_error(id: i64, message: &str) -> serde_json::Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": -32600, "message": message }
    })
}

fn json_rpc_error_value(id: &serde_json::Value, message: &str, code: i64) -> serde_json::Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
}
