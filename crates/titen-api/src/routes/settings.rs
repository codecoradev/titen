use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use tracing::{error, warn};

use crate::server::{AppState, error_response};
use titen_core::models::{AppSettingsResponse, UpdateAppSettings};

/// Derive the OAuth redirect URI from a trusted source.
///
/// Priority:
/// 1. `TITEN_OAUTH_REDIRECT_URI` env var (explicit, safest)
/// 2. `APP_URL` env var (already used for CORS and canonical URL)
/// 3. Host header — ONLY if it matches an entry in `TITEN_ALLOWED_HOSTS`
///    allowlist (comma-separated, e.g. "titen.ajianaz.dev,localhost:7845")
///
/// We never blindly trust the Host header because it can be spoofed by
/// clients, leading to OAuth redirect-code interception.
fn derive_redirect_uri(headers: &HeaderMap) -> String {
    // 1. Explicit env var — always wins
    if let Ok(env_uri) = std::env::var("TITEN_OAUTH_REDIRECT_URI") {
        if !env_uri.is_empty() {
            return env_uri;
        }
    }

    // 2. APP_URL — already the canonical public URL, trusted by config
    if let Ok(app_url) = std::env::var("APP_URL") {
        let app_url = app_url.trim_end_matches('/');
        if !app_url.is_empty() {
            // Enforce HTTPS for non-localhost (OAuth security requirement)
            let is_localhost =
                app_url.starts_with("http://localhost") || app_url.starts_with("http://127.0.0.1");
            if app_url.starts_with("https://") || is_localhost {
                return format!("{app_url}/auth/callback");
            } else {
                warn!(
                    "APP_URL '{app_url}' is not HTTPS; refusing to use for OAuth redirect URI. \
                     Use https:// or set TITEN_OAUTH_REDIRECT_URI explicitly."
                );
            }
        }
    }

    // 3. Host header — only if allowlisted
    let allowed = std::env::var("TITEN_ALLOWED_HOSTS").unwrap_or_default();
    let allowed_hosts: Vec<&str> = allowed
        .split(',')
        .map(|h| h.trim())
        .filter(|h| !h.is_empty())
        .collect();

    if allowed_hosts.is_empty() {
        // No allowlist configured — refuse to derive from Host header
        warn!(
            "TITEN_OAUTH_REDIRECT_URI and APP_URL not set, TITEN_ALLOWED_HOSTS is empty; \
             OAuth redirect URI will be empty. Configure one of these env vars."
        );
        return String::new();
    }

    let host = headers
        .get(axum::http::header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    if allowed_hosts.contains(&host) {
        let scheme = if let Some(h) = headers.get("x-forwarded-proto") {
            h.to_str().unwrap_or("https")
        } else {
            "https"
        };
        format!("{scheme}://{host}/auth/callback")
    } else {
        warn!(
            "Host '{host}' not in TITEN_ALLOWED_HOSTS allowlist; \
             refusing to derive redirect URI"
        );
        String::new()
    }
}

/// GET /api/settings — return app settings.
///
/// The `threads_app_secret` is NEVER returned to the client.
/// Instead, `threads_app_secret_set` (bool) indicates whether a secret is stored.
pub async fn get_settings(State(state): State<AppState>) -> (StatusCode, Json<serde_json::Value>) {
    match state.store.get_app_settings().await {
        Ok(settings) => {
            let response = AppSettingsResponse {
                instance_name: settings.instance_name,
                auto_fetch_comments: settings.auto_fetch_comments,
                comment_fetch_interval: settings.comment_fetch_interval,
                schedule_lookahead_hours: settings.schedule_lookahead_hours,
                threads_app_id: settings.threads_app_id,
                threads_app_secret_set: settings
                    .threads_app_secret_enc
                    .as_ref()
                    .is_some_and(|s| !s.is_empty()),
            };
            (
                StatusCode::OK,
                Json(serde_json::json!({ "data": response })),
            )
        }
        Err(e) => {
            error!(target: "titen::settings", "SETTINGS_GET_FAIL {}", e);
            let (status, body) = error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "SETTINGS_GET_FAILED",
                &e.to_string(),
            );
            (
                status,
                Json(serde_json::json!({ "error": body.error, "code": body.code })),
            )
        }
    }
}

/// PUT /api/settings — update app settings.
///
/// Only provided fields are updated. If `threads_app_secret` is `None`,
/// the existing secret is preserved. If `Some("")`, the secret is cleared.
pub async fn update_settings(
    State(state): State<AppState>,
    Json(input): Json<UpdateAppSettings>,
) -> (StatusCode, Json<serde_json::Value>) {
    match state.store.update_app_settings(&input).await {
        Ok(settings) => {
            let response = AppSettingsResponse {
                instance_name: settings.instance_name,
                auto_fetch_comments: settings.auto_fetch_comments,
                comment_fetch_interval: settings.comment_fetch_interval,
                schedule_lookahead_hours: settings.schedule_lookahead_hours,
                threads_app_id: settings.threads_app_id,
                threads_app_secret_set: settings
                    .threads_app_secret_enc
                    .as_ref()
                    .is_some_and(|s| !s.is_empty()),
            };
            (
                StatusCode::OK,
                Json(serde_json::json!({ "data": response })),
            )
        }
        Err(e) => {
            error!(target: "titen::settings", "SETTINGS_UPDATE_FAIL {}", e);
            let (status, body) = error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "SETTINGS_UPDATE_FAILED",
                &e.to_string(),
            );
            (
                status,
                Json(serde_json::json!({ "error": body.error, "code": body.code })),
            )
        }
    }
}

/// GET /api/settings/oauth-config — return OAuth configuration for the frontend.
///
/// Returns only the `app_id` and constructs the Meta authorize URL.
/// The `app_secret` is never included in the response.
pub async fn get_oauth_config(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> (StatusCode, Json<serde_json::Value>) {
    match state.store.get_app_settings().await {
        Ok(settings) => {
            let redirect_uri = derive_redirect_uri(&headers);

            // Only generate authorize_url if redirect_uri is non-empty.
            // If empty, frontend will construct the URL client-side using
            // window.location.origin (more reliable than internal Host header).
            let authorize_url = if redirect_uri.is_empty() {
                None
            } else {
                settings.threads_app_id.as_ref().map(|app_id| {
                    format!(
                        "https://threads.net/oauth/authorize?client_id={}&redirect_uri={}&scope=threads_basic,threads_content_publish,threads_manage_replies,threads_manage_mentions,threads_keyword_search,threads_profile_discovery,threads_share_to_instagram,threads_location_tagging&response_type=code",
                        urlencode(app_id),
                        urlencode(&redirect_uri),
                    )
                })
            };

            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "data": {
                        "app_id": settings.threads_app_id,
                        "redirect_uri": redirect_uri,
                        "authorize_url": authorize_url,
                        "secret_configured": settings
                            .threads_app_secret_enc
                            .as_ref()
                            .is_some_and(|s| !s.is_empty()),
                    }
                })),
            )
        }
        Err(e) => {
            warn!(target: "titen::settings", "OAUTH_CONFIG_FAIL {}", e);
            let (status, body) = error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "OAUTH_CONFIG_FAILED",
                &e.to_string(),
            );
            (
                status,
                Json(serde_json::json!({ "error": body.error, "code": body.code })),
            )
        }
    }
}

/// Minimal percent-encoder for OAuth URL parameters.
fn urlencode(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}
