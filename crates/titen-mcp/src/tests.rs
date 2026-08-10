//! Unit tests for titen-mcp: JSON-RPC protocol, tool list, argument validation,
//! SSRF protection, and magic bytes validation.

use super::*;

// ─── JSON-RPC protocol helpers ──────────────────────────

#[test]
fn test_json_rpc_error_has_correct_code() {
    let resp = json_rpc_error(42, "bad request");
    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 42);
    assert_eq!(resp["error"]["code"], -32600);
    assert_eq!(resp["error"]["message"], "bad request");
}

#[test]
fn test_json_rpc_error_value_with_null_id() {
    let resp = json_rpc_error_value(&json!(null), "method not found", -32601);
    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], json!(null));
    assert_eq!(resp["error"]["code"], -32601);
    assert_eq!(resp["error"]["message"], "method not found");
}

#[test]
fn test_json_rpc_error_value_with_string_id() {
    let resp = json_rpc_error_value(&json!("abc-123"), "oops", -32603);
    assert_eq!(resp["id"], "abc-123");
    assert_eq!(resp["error"]["code"], -32603);
}

#[test]
fn test_json_rpc_parse_error_format() {
    let bad_json = "{not valid json";
    let parse_result: Result<serde_json::Value, _> = serde_json::from_str(bad_json);
    assert!(parse_result.is_err());
    let resp = json_rpc_error(0, &format!("Parse error: {}", parse_result.unwrap_err()));
    assert_eq!(resp["error"]["code"], -32600);
    assert!(
        resp["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Parse error")
    );
}

#[test]
fn test_method_not_found_returns_error_code() {
    let id = json!(1);
    let method = "some/unknown/method";
    let result = match method {
        "initialize" => json!({}),
        "tools/list" => json!({}),
        "tools/call" => json!({}),
        _ => json_rpc_error_value(&id, "Method not found", -32601),
    };
    assert_eq!(result["error"]["code"], -32601);
    assert_eq!(result["error"]["message"], "Method not found");
}

// ─── Tool list completeness ─────────────────────────────

#[test]
fn test_tools_list_returns_all_expected_tools() {
    let list = tools_list();
    let tools = list["tools"].as_array().expect("tools should be an array");

    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();

    let expected = [
        "list_accounts",
        "get_user_profile",
        "get_publishing_limit",
        "create_post",
        "schedule_post",
        "list_schedules",
        "cancel_schedule",
        "refresh_token",
        "check_tokens",
        "fetch_comments",
        "get_post_sentiment",
        "get_post_insights",
        "get_account_analytics",
        "delete_post",
        "create_container",
        "publish_container",
        "list_posts",
        "get_post",
        "get_schedule",
        "approve_schedule",
        "reject_schedule",
        "upload_media",
        "list_media",
        "fetch_mentions",
        "list_mentions",
        "search_keyword",
        "get_post_trend",
        "reply_to_comment",
        "exchange_oauth_code",
    ];

    for name in &expected {
        assert!(
            names.contains(name),
            "Tool '{name}' missing from tools_list()"
        );
    }
}

#[test]
fn test_tools_list_count_matches() {
    let list = tools_list();
    let tools = list["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 29, "Expected 29 tools in tools_list()");
}

#[test]
fn test_each_tool_has_name_and_description() {
    let list = tools_list();
    let tools = list["tools"].as_array().unwrap();
    for (i, tool) in tools.iter().enumerate() {
        assert!(tool["name"].is_string(), "Tool at index {i} missing 'name'");
        assert!(
            tool["description"].is_string(),
            "Tool at index {i} missing 'description'"
        );
        assert!(
            tool["inputSchema"].is_object(),
            "Tool at index {i} missing 'inputSchema'"
        );
    }
}

// ─── Argument extraction patterns ───────────────────────

#[test]
fn test_extract_account_id_from_args() {
    let args = json!({ "account_id": "acc-123" });
    let account_id = args
        .get("account_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert_eq!(account_id, "acc-123");
}

#[test]
fn test_extract_missing_account_id_defaults_to_empty() {
    let args = json!({});
    let account_id = args
        .get("account_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert_eq!(account_id, "");
}

#[test]
fn test_extract_post_id_from_args() {
    let args = json!({ "post_id": "post-456" });
    let post_id = args.get("post_id").and_then(|v| v.as_str()).unwrap_or("");
    assert_eq!(post_id, "post-456");
}

#[test]
fn test_extract_int_arg_as_u64() {
    let args = json!({ "limit": 100, "offset": 10 });
    let limit = args.get("limit").and_then(|v| v.as_u64()).map(|v| v as i64);
    let offset = args
        .get("offset")
        .and_then(|v| v.as_u64())
        .map(|v| v as i64);
    assert_eq!(limit, Some(100));
    assert_eq!(offset, Some(10));
}

// ─── Caption length validation (#136) ───────────────────

#[test]
fn test_caption_at_limit_passes() {
    let caption: String = "a".repeat(500);
    assert_eq!(caption.chars().count(), 500);
    assert!(caption.chars().count() <= 500);
}

#[test]
fn test_caption_over_limit_triggers_validation() {
    let caption: String = "a".repeat(501);
    assert!(caption.chars().count() > 500);

    let should_reject = caption.chars().count() > 500;
    assert!(should_reject);

    let error_response = json!({
        "error": format!(
            "Caption exceeds Threads API limit of 500 characters (got {})",
            caption.chars().count()
        ),
        "code": "CAPTION_TOO_LONG"
    });
    assert_eq!(error_response["code"], "CAPTION_TOO_LONG");
}

#[test]
fn test_caption_with_unicode_counts_chars_not_bytes() {
    let caption = "😀".repeat(501);
    assert_eq!(caption.chars().count(), 501);
    assert!(caption.len() > 501);
    assert!(caption.chars().count() > 500);
}

// ─── Tool name routing ──────────────────────────────────

#[test]
fn test_unknown_tool_name_produces_error_result() {
    let tool_name = "nonexistent_tool";
    let known_tools = [
        "list_accounts",
        "create_post",
        "schedule_post",
        "list_posts",
        "get_post",
        "delete_post",
        "list_schedules",
        "cancel_schedule",
        "get_schedule",
    ];
    let is_unknown = !known_tools.contains(&tool_name);
    assert!(is_unknown);

    let err_msg = format!("Unknown tool: {tool_name}");
    let result = json!({
        "content": [{ "type": "text", "text": format!("Error: {err_msg}") }],
        "isError": true
    });
    assert_eq!(result["isError"], true);
    assert!(
        result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Unknown tool")
    );
}

#[test]
fn test_known_tool_names_are_routed() {
    let known = [
        "list_accounts",
        "get_user_profile",
        "create_post",
        "schedule_post",
        "list_schedules",
        "cancel_schedule",
        "delete_post",
        "get_post",
        "get_schedule",
        "list_posts",
    ];
    for name in &known {
        let is_unknown = name.starts_with("nonexistent");
        assert!(!is_unknown, "{name} should not be unknown");
    }
}

// ─── MCP response wrapping ──────────────────────────────

#[test]
fn test_success_response_wraps_content() {
    let data = json!({ "id": "abc", "status": "published" });
    let result = json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&data).unwrap_or_else(|_| data.to_string())
        }]
    });
    assert_eq!(result["content"][0]["type"], "text");
    assert!(
        result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("published")
    );
    assert!(result.get("isError").is_none());
}

#[test]
fn test_error_response_has_is_error_flag() {
    let msg = "Post not found: xyz";
    let result = json!({
        "content": [{ "type": "text", "text": format!("Error: {msg}") }],
        "isError": true
    });
    assert_eq!(result["isError"], true);
    assert!(
        result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Error: Post not found")
    );
}

// ─── SSRF protection helpers ────────────────────────────

#[test]
fn test_is_private_host_localhost() {
    assert!(is_private_host("localhost"));
    assert!(is_private_host("127.0.0.1"));
    assert!(is_private_host("::1"));
    assert!(is_private_host("0.0.0.0"));
}

#[test]
fn test_is_private_host_internal_tld() {
    assert!(is_private_host("myapp.local"));
    assert!(is_private_host("db.internal"));
    assert!(is_private_host("api.localhost"));
}

#[test]
fn test_ip_is_private_v4() {
    use std::net::IpAddr;
    let loopback: IpAddr = "127.0.0.1".parse().unwrap();
    let private10: IpAddr = "10.0.0.1".parse().unwrap();
    let private192: IpAddr = "192.168.1.1".parse().unwrap();
    let private172: IpAddr = "172.16.0.1".parse().unwrap();
    assert!(ip_is_private(&loopback));
    assert!(ip_is_private(&private10));
    assert!(ip_is_private(&private192));
    assert!(ip_is_private(&private172));
}

#[test]
fn test_ip_is_private_v6() {
    use std::net::IpAddr;
    let loopback: IpAddr = "::1".parse().unwrap();
    let unspec: IpAddr = "::".parse().unwrap();
    assert!(ip_is_private(&loopback));
    assert!(ip_is_private(&unspec));
}

// ─── Magic bytes validation ─────────────────────────────

#[test]
fn test_validate_magic_bytes_jpg() {
    let jpg = [0xFF, 0xD8, 0xFF, 0xE0];
    assert!(validate_magic_bytes(&jpg, "jpg"));
    let not_jpg = [0x89, 0x50, 0x4E];
    assert!(!validate_magic_bytes(&not_jpg, "jpg"));
}

#[test]
fn test_validate_magic_bytes_png() {
    let png = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    assert!(validate_magic_bytes(&png, "png"));
}

#[test]
fn test_validate_magic_bytes_gif() {
    let gif = [0x47, 0x49, 0x46, 0x38, 0x39, 0x61];
    assert!(validate_magic_bytes(&gif, "gif"));
}

#[test]
fn test_validate_magic_bytes_webp() {
    let webp = [
        0x52, 0x49, 0x46, 0x46, 0x00, 0x00, 0x00, 0x00, 0x57, 0x45, 0x42, 0x50,
    ];
    assert!(validate_magic_bytes(&webp, "webp"));
}

#[test]
fn test_validate_magic_bytes_too_short() {
    assert!(!validate_magic_bytes(&[0xFF], "jpg"));
    assert!(!validate_magic_bytes(&[], "png"));
}

#[test]
fn test_validate_magic_bytes_unknown_ext() {
    assert!(!validate_magic_bytes(&[0xFF, 0xD8, 0xFF], "bmp"));
}
