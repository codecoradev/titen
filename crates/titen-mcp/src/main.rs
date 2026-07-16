use std::io::{self, BufRead, Write};

fn main() {
    eprintln!("titen-mcp starting (stdio JSON-RPC)");

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

        let id = request
            .get("id")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let _params = request
            .get("params")
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        let result = match method {
            "initialize" => {
                serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": {} },
                    "serverInfo": {
                        "name": "titen",
                        "version": "0.1.0"
                    }
                })
            }
            "tools/list" => {
                serde_json::json!({
                    "tools": [
                        { "name": "list_accounts", "description": "List all Threads accounts managed by titen" },
                        { "name": "create_post", "description": "Create and publish a Threads post" },
                        { "name": "schedule_post", "description": "Schedule a post for future publishing" },
                        { "name": "list_schedules", "description": "List scheduled posts" },
                        { "name": "fetch_comments", "description": "Fetch comments from a Threads post" },
                        { "name": "get_post_sentiment", "description": "Get sentiment analysis for a post's comments" },
                        { "name": "get_post_analytics", "description": "Get analytics for a specific post" },
                        { "name": "get_account_analytics", "description": "Get analytics for an account's posts" },
                        { "name": "upload_media", "description": "Upload a media file to S3 storage" },
                        { "name": "refresh_token", "description": "Refresh an account's Threads token" },
                        { "name": "check_tokens", "description": "Check all accounts' token expiry status" },
                        { "name": "cancel_schedule", "description": "Cancel a scheduled post" },
                        { "name": "delete_post", "description": "Delete a post on Threads" }
                    ]
                })
            }
            "tools/call" => {
                let tool_name = request
                    .pointer("/params/name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                serde_json::json!({
                    "content": [{ "type": "text", "text": format!("Tool '{tool_name}' not yet implemented. Connect titen to a database to enable.") }],
                })
            }
            _ => json_rpc_error_value(&id, "Method not found", -32601),
        };

        let response = serde_json::json!({
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

fn json_rpc_error(id: i64, message: &str) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": -32600, "message": message }
    })
}

fn json_rpc_error_value(id: &serde_json::Value, message: &str, code: i64) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
}
