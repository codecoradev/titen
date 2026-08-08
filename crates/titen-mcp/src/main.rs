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
use titen_core::{
    Store, ThreadsClient,
    models::{CommentFilter, PostFilter, ScheduleFilter},
};

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
            },
            {
                "name": "list_posts",
                "description": "List published/draft posts with optional filtering",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "account_id": { "type": "string", "description": "Filter by account ID" },
                        "status": { "type": "string", "enum": ["draft", "published", "failed", "deleted"], "description": "Filter by post status" },
                        "limit": { "type": "integer", "description": "Max results (default 50, max 1000)" },
                        "offset": { "type": "integer", "description": "Pagination offset (default 0)" }
                    }
                }
            },
            {
                "name": "get_post",
                "description": "Get a single post by ID",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "post_id": { "type": "string", "description": "Post ID" }
                    },
                    "required": ["post_id"]
                }
            },
            {
                "name": "get_schedule",
                "description": "Get a single scheduled post by ID",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Schedule ID" }
                    },
                    "required": ["id"]
                }
            },
            {
                "name": "approve_schedule",
                "description": "Approve a pending scheduled post for publishing (HITL approval)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Schedule ID" },
                        "approved_by": { "type": "string", "description": "Who approved (optional)" }
                    },
                    "required": ["id"]
                }
            },
            {
                "name": "reject_schedule",
                "description": "Reject a pending scheduled post with optional reason (HITL rejection)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Schedule ID" },
                        "reason": { "type": "string", "description": "Rejection reason" }
                    },
                    "required": ["id"]
                }
            },
            {
                "name": "upload_media",
                "description": "Upload a media asset (image) to titen storage for use in posts/carousels",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "url": { "type": "string", "description": "Source URL of the image to upload" },
                        "alt_text": { "type": "string", "description": "Alt text for accessibility" }
                    },
                    "required": ["url"]
                }
            },
            {
                "name": "list_media",
                "description": "List media assets stored in titen",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "media_type": { "type": "string", "description": "Filter by media type (image, video)" },
                        "limit": { "type": "integer", "description": "Max results (default 50)" },
                        "offset": { "type": "integer", "description": "Pagination offset" }
                    }
                }
            },
            {
                "name": "fetch_mentions",
                "description": "Fetch mentions of a managed account from the Threads API and store them",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "account_id": { "type": "string", "description": "Account ID to fetch mentions for" }
                    },
                    "required": ["account_id"]
                }
            },
            {
                "name": "list_mentions",
                "description": "List stored mentions for an account",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "account_id": { "type": "string", "description": "Account ID" },
                        "limit": { "type": "integer", "description": "Max results (default 50)" },
                        "offset": { "type": "integer", "description": "Pagination offset" }
                    },
                    "required": ["account_id"]
                }
            },
            {
                "name": "search_keyword",
                "description": "Search Threads for a keyword or trending topic",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "account_id": { "type": "string", "description": "Account ID to authenticate as" },
                        "keyword": { "type": "string", "description": "Search query" }
                    },
                    "required": ["account_id", "keyword"]
                }
            },
            {
                "name": "get_post_trend",
                "description": "Get time-series engagement trend data for a post (stored analytics snapshots)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "post_id": { "type": "string", "description": "Post ID" }
                    },
                    "required": ["post_id"]
                }
            },
            {
                "name": "reply_to_comment",
                "description": "Reply to a comment on a Threads post directly from the AI agent",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "post_id": { "type": "string", "description": "The titen post ID" },
                        "comment_id": { "type": "string", "description": "The Threads comment ID to reply to" },
                        "text": { "type": "string", "description": "Reply text" }
                    },
                    "required": ["post_id", "comment_id", "text"]
                }
            },
            {
                "name": "exchange_oauth_code",
                "description": "Exchange an OAuth authorization code for a long-lived token and add account to titen",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "code": { "type": "string", "description": "OAuth authorization code from Meta" }
                    },
                    "required": ["code"]
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
                auto_approve: false,
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
            match store
                .list_schedules(&ScheduleFilter {
                    account_id: args
                        .get("account_id")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    status: args
                        .get("status")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    ..Default::default()
                })
                .await
            {
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
            let comments = match store
                .list_comments(post_id, &CommentFilter::default())
                .await
            {
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
                .list_posts(&PostFilter {
                    account_id: Some(account_id.to_string()),
                    status: Some("published".to_string()),
                    limit: Some(100),
                    ..Default::default()
                })
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
        "list_posts" => rt.block_on(async {
            match store
                .list_posts(&PostFilter {
                    account_id: args
                        .get("account_id")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    status: args
                        .get("status")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    limit: args.get("limit").and_then(|v| v.as_u64()).map(|v| v as i64),
                    offset: args
                        .get("offset")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as i64),
                    ..Default::default()
                })
                .await
            {
                Ok(posts) => {
                    let data: Vec<serde_json::Value> = posts
                        .into_iter()
                        .map(|p| {
                            json!({
                                "id": p.id,
                                "account_id": p.account_id,
                                "caption": p.caption,
                                "status": p.status,
                                "media_type": p.media_type,
                                "threads_post_id": p.threads_post_id,
                                "created_at": p.created_at,
                            })
                        })
                        .collect();
                    Ok(json!(data))
                }
                Err(e) => Err(format!("Failed to list posts: {e}")),
            }
        }),
        "get_post" => rt.block_on(async {
            let post_id = args.get("post_id").and_then(|v| v.as_str()).unwrap_or("");
            match store.get_post(post_id).await {
                Ok(post) => Ok(json!({
                    "id": post.id,
                    "account_id": post.account_id,
                    "caption": post.caption,
                    "status": post.status,
                    "media_type": post.media_type,
                    "threads_post_id": post.threads_post_id,
                    "published_at": post.published_at,
                    "created_at": post.created_at,
                })),
                Err(e) => Err(format!("Post not found: {e}")),
            }
        }),
        "get_schedule" => rt.block_on(async {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            match store.get_schedule(id).await {
                Ok(s) => Ok(json!({
                    "id": s.id,
                    "account_id": s.account_id,
                    "caption": s.caption,
                    "scheduled_at": s.scheduled_at,
                    "status": s.status,
                    "media_type": s.media_type,
                })),
                Err(e) => Err(format!("Schedule not found: {e}")),
            }
        }),
        "approve_schedule" => rt.block_on(async {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let approved_by = args.get("approved_by").and_then(|v| v.as_str());
            match store.approve_schedule(id, approved_by).await {
                Ok(s) => Ok(json!({
                    "id": s.id,
                    "status": s.status,
                    "approved_by": s.approved_by,
                })),
                Err(e) => Err(format!("Failed to approve schedule: {e}")),
            }
        }),
        "reject_schedule" => rt.block_on(async {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let reason = args.get("reason").and_then(|v| v.as_str());
            match store.reject_schedule(id, reason).await {
                Ok(s) => Ok(json!({
                    "id": s.id,
                    "status": s.status,
                    "error": s.error,
                })),
                Err(e) => Err(format!("Failed to reject schedule: {e}")),
            }
        }),
        "upload_media" => rt.block_on(async {
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
            let filename = args
                .get("filename")
                .and_then(|v| v.as_str())
                .unwrap_or("upload");

            // Validate URL scheme — prevent SSRF and file:// attacks
            let parsed = match reqwest::Url::parse(url) {
                Ok(u) => u,
                Err(_) => return Err("Invalid URL format".to_string()),
            };
            if parsed.scheme() != "https" && parsed.scheme() != "http" {
                return Err(format!(
                    "URL scheme '{}' not allowed. Only http/https.",
                    parsed.scheme()
                ));
            }
            // Check host is not private/internal (SSRF protection)
            if let Some(host) = parsed.host_str() {
                if is_private_host(host) {
                    return Err("URL points to a private/internal host. Blocked for security.".to_string());
                }
            }

            // HTTP client (no redirect following to prevent SSRF via redirect)
            let client = reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

            let mut resp = match client.get(url).send().await {
                Ok(r) => r,
                Err(e) => return Err(format!("Failed to download image: {e}")),
            };
            if !resp.status().is_success() {
                return Err(format!("Download failed: HTTP {}", resp.status()));
            }
            let content_type = resp
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("image/jpeg")
                .to_string();

            // Validate content-type is an allowed image type
            let ext = match content_type.as_str() {
                "image/jpeg" | "image/jpg" => "jpg",
                "image/png" => "png",
                "image/gif" => "gif",
                "image/webp" => "webp",
                _ => {
                    return Err(format!(
                        "Unsupported content type '{content_type}'. Only image/jpeg, image/png, image/gif, image/webp are allowed."
                    ));
                }
            };

            // Check content-length before downloading body
            if let Some(len) = resp.content_length() {
                if len > 50 * 1024 * 1024 {
                    return Err("File too large. Maximum 50MB.".to_string());
                }
            }

            // Stream body with hard cap (prevents chunked-encoding OOM bypass)
            const MAX_SIZE: usize = 50 * 1024 * 1024;
            let mut buf: Vec<u8> = Vec::with_capacity(1024 * 1024);
            loop {
                match resp.chunk().await {
                    Ok(Some(chunk)) => {
                        if buf.len() + chunk.len() > MAX_SIZE {
                            return Err(
                                "File exceeds 50MB streaming limit.".to_string(),
                            );
                        }
                        buf.extend_from_slice(&chunk);
                    }
                    Ok(None) => break, // EOF
                    Err(e) => return Err(format!("Download failed: {e}")),
                }
            }

            // Validate magic bytes match declared content-type (prevent spoofing)
            if !validate_magic_bytes(&buf, ext) {
                return Err(format!(
                    "File content does not match declared type '{ext}'. Possible spoofing attempt."
                ));
            }

            // Upload to S3
            use titen_core::storage::Storage;
            let s3 = match titen_core::storage::S3Storage::from_env() {
                Ok(s) => s,
                Err(_) => return Err("S3 storage not configured. Set S3_* env vars.".to_string()),
            };
            let asset_id = uuid::Uuid::now_v7();
            let s3_key = format!("uploads/{asset_id}.{ext}");
            let s3_url = match s3.upload(&s3_key, &buf, &content_type).await {
                Ok(u) => u,
                Err(e) => return Err(format!("Failed to upload to S3: {e}")),
            };

            let id = asset_id.to_string();
            match store
                .create_media_asset(
                    &id,
                    filename,
                    &content_type,
                    buf.len() as i64,
                    &s3_key,
                    Some(&s3_url),
                )
                .await
            {
                Ok(asset) => Ok(json!({
                    "id": asset.id,
                    "filename": asset.filename,
                    "s3_key": asset.s3_key,
                    "s3_url": asset.s3_url,
                    "content_type": asset.content_type,
                    "size_bytes": asset.size_bytes,
                    "uploaded_at": asset.uploaded_at,
                })),
                Err(e) => {
                    // Best-effort S3 cleanup on DB insert failure
                    let _ = s3.delete(&s3_key).await;
                    Err(format!("Failed to create media asset: {e}"))
                }
            }
        }),
        "list_media" => rt.block_on(async {
            match store
                .list_media(&titen_core::models::MediaFilter {
                    content_type: args
                        .get("content_type")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    limit: args.get("limit").and_then(|v| v.as_u64()).map(|v| v as i64),
                    offset: args
                        .get("offset")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as i64),
                    ..Default::default()
                })
                .await
            {
                Ok(assets) => {
                    let data: Vec<serde_json::Value> = assets
                        .into_iter()
                        .map(|a| {
                            json!({
                                "id": a.id,
                                "filename": a.filename,
                                "s3_key": a.s3_key,
                                "s3_url": a.s3_url,
                                "content_type": a.content_type,
                                "size_bytes": a.size_bytes,
                            })
                        })
                        .collect();
                    Ok(json!(data))
                }
                Err(e) => Err(format!("Failed to list media: {e}")),
            }
        }),
        "fetch_mentions" => rt.block_on(async {
            let account_id = args
                .get("account_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let limit = args.get("limit").and_then(|v| v.as_u64()).map(|v| v as u32);

            let account = match store.get_account(account_id).await {
                Ok(a) => a,
                Err(e) => return Err(format!("Account not found: {e}")),
            };

            // Fetch mentions from Threads API (returns raw JSON)
            let mentions = match threads_client.fetch_mentions(&account, limit).await {
                Ok(m) => m,
                Err(e) => return Err(format!("Failed to fetch mentions: {e}")),
            };

            let mut stored = 0;
            for mention in &mentions {
                let id = uuid::Uuid::now_v7().to_string();
                let m = titen_core::models::Mention {
                    id,
                    account_id: account_id.to_string(),
                    threads_mention_id: mention
                        .get("id")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    author_username: mention
                        .get("username")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    author_user_id: None,
                    text: mention
                        .get("text")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    media_type: mention
                        .get("media_type")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    permalink: mention
                        .get("permalink")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    mentioned_at: mention
                        .get("timestamp")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    fetched_at: chrono::Utc::now().to_rfc3339(),
                };
                if store.upsert_mention(&m).await.is_ok() {
                    stored += 1;
                }
            }

            Ok(json!({
                "fetched": mentions.len(),
                "stored": stored,
            }))
        }),
        "list_mentions" => rt.block_on(async {
            let account_id = args
                .get("account_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            match store
                .list_mentions(&titen_core::models::MentionFilter {
                    account_id: Some(account_id.to_string()),
                    limit: args.get("limit").and_then(|v| v.as_u64()).map(|v| v as i64),
                    offset: args
                        .get("offset")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as i64),
                    ..Default::default()
                })
                .await
            {
                Ok(mentions) => {
                    let data: Vec<serde_json::Value> = mentions
                        .into_iter()
                        .map(|m| {
                            json!({
                                "id": m.id,
                                "author_username": m.author_username,
                                "text": m.text,
                                "permalink": m.permalink,
                                "mentioned_at": m.mentioned_at,
                            })
                        })
                        .collect();
                    Ok(json!(data))
                }
                Err(e) => Err(format!("Failed to list mentions: {e}")),
            }
        }),
        "search_keyword" => rt.block_on(async {
            let account_id = args
                .get("account_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let keyword = args.get("keyword").and_then(|v| v.as_str()).unwrap_or("");

            let account = match store.get_account(account_id).await {
                Ok(a) => a,
                Err(e) => return Err(format!("Account not found: {e}")),
            };

            match threads_client.search_keyword(&account, keyword, None).await {
                Ok(results) => Ok(json!(results)),
                Err(e) => Err(format!("Search failed: {e}")),
            }
        }),
        "get_post_trend" => rt.block_on(async {
            let post_id = args.get("post_id").and_then(|v| v.as_str()).unwrap_or("");
            match store.list_analytics_snap(post_id).await {
                Ok(snaps) => {
                    let data: Vec<serde_json::Value> = snaps
                        .into_iter()
                        .map(|s| {
                            json!({
                                "views": s.views,
                                "likes": s.likes,
                                "replies": s.replies,
                                "reposts": s.reposts,
                                "quotes": s.quotes,
                                "snapshot_at": s.snapshot_at,
                            })
                        })
                        .collect();
                    Ok(json!({ "post_id": post_id, "snapshots": data, "count": data.len() }))
                }
                Err(e) => Err(format!("Failed to get trend data: {e}")),
            }
        }),
        "reply_to_comment" => rt.block_on(async {
            let post_id = args.get("post_id").and_then(|v| v.as_str()).unwrap_or("");
            let comment_id = args
                .get("comment_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");

            let post = match store.get_post(post_id).await {
                Ok(p) => p,
                Err(e) => return Err(format!("Post not found: {e}")),
            };

            // reply_to_comment: verify post exists and is published
            if post.threads_post_id.is_none() {
                return Err("Post not yet published to Threads".to_string());
            }

            let account = match store.get_account(&post.account_id).await {
                Ok(a) => a,
                Err(e) => return Err(format!("Account not found: {e}")),
            };

            match threads_client
                .create_reply(&account, comment_id, text)
                .await
            {
                Ok(reply_id) => Ok(json!({
                    "reply_id": reply_id,
                    "comment_id": comment_id,
                    "post_id": post_id,
                })),
                Err(e) => Err(format!("Failed to reply: {e}")),
            }
        }),
        "exchange_oauth_code" => rt.block_on(async {
            let code = args.get("code").and_then(|v| v.as_str()).unwrap_or("");

            // Read OAuth credentials from env — never accept from MCP client (security)
            let client_id = std::env::var("THREADS_CLIENT_ID")
                .unwrap_or_else(|_| {
                    eprintln!("WARNING: THREADS_CLIENT_ID not set");
                    String::new()
                });
            let client_secret = std::env::var("THREADS_CLIENT_SECRET")
                .unwrap_or_else(|_| {
                    eprintln!("WARNING: THREADS_CLIENT_SECRET not set");
                    String::new()
                });
            let redirect_uri = std::env::var("THREADS_REDIRECT_URI")
                .unwrap_or_else(|_| {
                    eprintln!("ERROR: THREADS_REDIRECT_URI not set — OAuth exchange will fail");
                    String::new()
                });

            if client_id.is_empty() || client_secret.is_empty() {
                return Err("OAuth credentials not configured on server. Set THREADS_CLIENT_ID and THREADS_CLIENT_SECRET env vars.".to_string());
            }

            match threads_client
                .exchange_code_for_token(code, &client_id, &client_secret, &redirect_uri)
                .await
            {
                Ok((access_token, _token_type)) => {
                    // Resolve user identity from token
                    match threads_client.resolve_account(&access_token).await {
                        Ok((user_id, username)) => {
                            let id = uuid::Uuid::now_v7().to_string();
                            let expires_at = (chrono::Utc::now()
                                + chrono::Duration::seconds(5184000))
                            .to_rfc3339(); // ~60 days
                            let input = titen_core::models::CreateAccount {
                                username: Some(username.clone()),
                                user_id: Some(user_id.clone()),
                                access_token: access_token.clone(),
                                expires_at,
                                app_id: Some(client_id.to_string()),
                                app_secret: None, // Never persist client secret — only needed at exchange time
                            };
                            match store.create_account(&id, &input).await {
                                Ok(account) => Ok(json!({
                                    "id": account.id,
                                    "username": account.username,
                                    "user_id": account.user_id,
                                    "token_status": account.token_status(),
                                })),
                                Err(e) => Err(format!("Account created but DB save failed: {e}")),
                            }
                        }
                        Err(e) => Err(format!("Token exchanged but profile resolve failed: {e}")),
                    }
                }
                Err(e) => Err(format!("OAuth code exchange failed: {e}")),
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

/// Check if a hostname is a private/internal IP or localhost (SSRF protection).
/// Also performs DNS resolution to catch hostnames resolving to private IPs.
fn is_private_host(host: &str) -> bool {
    use std::net::{IpAddr, ToSocketAddrs};

    // Localhost variants
    if host == "localhost" || host == "127.0.0.1" || host == "::1" || host == "0.0.0.0" {
        return true;
    }

    // Try parsing as literal IP address first
    if let Ok(ip) = host.parse::<IpAddr>() {
        if ip_is_private(&ip) {
            return true;
        }
    }

    // Block internal TLDs
    if host.ends_with(".local")
        || host.ends_with(".internal")
        || host.ends_with(".localhost")
        || host.ends_with(".arpa")
    {
        return true;
    }

    // DNS resolution: check ALL resolved IPs (prevents DNS rebinding attacks)
    let lookup = format!("{host}:0");
    if let Ok(addrs) = lookup.to_socket_addrs() {
        for addr in addrs {
            if ip_is_private(&addr.ip()) {
                return true;
            }
        }
    }

    false
}

/// Check if a resolved IP address is private/internal.
fn ip_is_private(ip: &std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            if v4.is_loopback()
                || v4.is_unspecified()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_documentation()
            {
                return true;
            }
            // Carrier-grade NAT (100.64.0.0/10)
            let o = v4.octets();
            o[0] == 100 && (o[1] & 0xc0) == 64
        }
        std::net::IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified() || v6.is_multicast(),
    }
}

/// Validate that file magic bytes match the declared image type (anti-spoofing).
fn validate_magic_bytes(data: &[u8], ext: &str) -> bool {
    if data.len() < 4 {
        return false;
    }
    match ext {
        "jpg" => data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF,
        "png" => {
            data[0] == 0x89
                && data[1] == 0x50
                && data[2] == 0x4E
                && data[3] == 0x47
                && data[4] == 0x0D
                && data[5] == 0x0A
                && data[6] == 0x1A
                && data[7] == 0x0A
        }
        "gif" => data[0] == 0x47 && data[1] == 0x49 && data[2] == 0x46 && data[3] == 0x38,
        "webp" => {
            data.len() >= 12
                && data[0] == 0x52
                && data[1] == 0x49
                && data[2] == 0x46
                && data[3] == 0x46
                && data[8] == 0x57
                && data[9] == 0x45
                && data[10] == 0x42
                && data[11] == 0x50
        }
        _ => false,
    }
}
