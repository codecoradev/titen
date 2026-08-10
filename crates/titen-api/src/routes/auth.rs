use axum::{
    Json,
    extract::{ConnectInfo, State},
    http::{HeaderMap, HeaderValue, StatusCode, header::SET_COOKIE},
    response::IntoResponse,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

use crate::server::{AppState, ErrorResponse};

/// Simple in-memory rate limiter for login attempts.
/// Tracks failed attempts per IP, blocks after MAX_ATTEMPTS within WINDOW.
const MAX_LOGIN_ATTEMPTS: u32 = 5;
const LOGIN_WINDOW: Duration = Duration::from_secs(60); // 1 minute
const LOGIN_LOCKOUT: Duration = Duration::from_secs(300); // 5 minutes
const MAX_TRACKED_IPS: usize = 10_000; // prevent unbounded memory growth

static LOGIN_ATTEMPTS: Mutex<Option<HashMap<String, Vec<Instant>>>> = Mutex::new(None);

/// Lock the mutex, recovering from poison by taking the inner data.
fn lock_attempts() -> std::sync::MutexGuard<'static, Option<HashMap<String, Vec<Instant>>>> {
    LOGIN_ATTEMPTS.lock().unwrap_or_else(|e| e.into_inner())
}

/// Check if an IP is rate-limited. Returns true if the IP should be blocked.
fn is_rate_limited(ip: &str) -> bool {
    let mut attempts = lock_attempts();
    let map = attempts.get_or_insert_with(HashMap::new);
    let now = Instant::now();

    // Prune entries older than the lockout window
    let timestamps = map.entry(ip.to_string()).or_default();
    timestamps.retain(|t| now.duration_since(*t) < LOGIN_LOCKOUT);

    // Count attempts within the sliding window
    let recent: Vec<_> = timestamps
        .iter()
        .filter(|t| now.duration_since(**t) < LOGIN_WINDOW)
        .collect();

    timestamps.len() >= MAX_LOGIN_ATTEMPTS as usize || recent.len() >= MAX_LOGIN_ATTEMPTS as usize
}

/// Record a failed login attempt for an IP.
fn record_failed_attempt(ip: &str) {
    let mut attempts = lock_attempts();
    let map = attempts.get_or_insert_with(HashMap::new);

    // Periodic cleanup: remove IPs with no recent attempts to prevent unbounded growth
    if map.len() > MAX_TRACKED_IPS {
        let now = Instant::now();
        map.retain(|_, ts| ts.iter().any(|t| now.duration_since(*t) < LOGIN_LOCKOUT));
    }

    map.entry(ip.to_string()).or_default().push(Instant::now());
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub api_key: String,
}

#[derive(serde::Serialize)]
struct LoginResponse {
    valid: bool,
}

/// POST /api/auth/login — validate API key and set httpOnly session cookie.
///
/// The cookie stores the API key itself (same as X-API-Key header would carry).
/// This is acceptable because:
/// 1. httpOnly prevents JS access (XSS-safe, unlike localStorage)
/// 2. SameSite=Lax prevents CSRF on POST while allowing OAuth callback redirects
/// 3. The API key is already a bearer-style secret
pub async fn login(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(input): Json<LoginRequest>,
) -> impl IntoResponse {
    let ip = addr.ip().to_string();

    // P2.1: Rate limiting — block IPs with too many failed attempts.
    if is_rate_limited(&ip) {
        warn!(target: "titen::auth", "LOGIN_RATE_LIMITED ip={}", ip);
        let body = ErrorResponse {
            error: "Too many login attempts. Please try again later.".to_string(),
            code: "RATE_LIMITED".to_string(),
        };
        return (StatusCode::TOO_MANY_REQUESTS, HeaderMap::new(), Json(body)).into_response();
    }

    info!(target: "titen::auth", "LOGIN_ATTEMPT ip={} key_len={}", ip, input.api_key.len());

    let required_key = match &state.api_key {
        Some(key) if !key.is_empty() => key,
        _ => {
            warn!(target: "titen::auth", "LOGIN_DEV_MODE no API key configured, accepting all");
            let cookie = "titen_session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0";
            let mut headers = HeaderMap::new();
            headers.insert(SET_COOKIE, HeaderValue::from_static(cookie));
            return (StatusCode::OK, headers, Json(LoginResponse { valid: true })).into_response();
        }
    };

    if subtle::ConstantTimeEq::ct_eq(input.api_key.as_bytes(), required_key.as_bytes()).into() {
        // P1.3: Auto-detect HTTPS from X-Forwarded-Proto or explicit env var.
        let is_https = headers
            .get("X-Forwarded-Proto")
            .and_then(|v| v.to_str().ok())
            .map(|s| s == "https")
            .unwrap_or(false);
        let secure_env = std::env::var("TITEN_COOKIE_SECURE")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);
        let secure = secure_env || is_https;
        let secure_attr = if secure { "; Secure" } else { "" };
        let cookie_value = format!(
            "titen_session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=604800{secure_attr}",
            input.api_key
        );
        let mut resp_headers = HeaderMap::new();
        match HeaderValue::from_str(&cookie_value) {
            Ok(val) => resp_headers.insert(SET_COOKIE, val),
            Err(_) => {
                warn!(target: "titen::auth", "LOGIN_REJECT invalid chars in API key for cookie");
                let body = ErrorResponse {
                    error: "API key contains invalid characters for cookie storage".to_string(),
                    code: "INVALID_API_KEY".to_string(),
                };
                return (StatusCode::BAD_REQUEST, HeaderMap::new(), Json(body)).into_response();
            }
        };
        info!(target: "titen::auth", "LOGIN_SUCCESS ip={} secure={} samesite=Lax max_age=604800", ip, secure);
        (
            StatusCode::OK,
            resp_headers,
            Json(LoginResponse { valid: true }),
        )
            .into_response()
    } else {
        warn!(target: "titen::auth", "LOGIN_FAIL ip={} key mismatch", ip);
        record_failed_attempt(&ip);
        let body = ErrorResponse {
            error: "Invalid API key".to_string(),
            code: "INVALID_API_KEY".to_string(),
        };
        (StatusCode::UNAUTHORIZED, HeaderMap::new(), Json(body)).into_response()
    }
}

/// GET /api/auth/session — check auth state.
///
/// Returns `requires_auth` (whether TITEN_API_KEY is set) and `authenticated`
/// (whether the request carries a valid session cookie).
pub async fn session(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let is_configured = state
        .api_key
        .as_ref()
        .map(|k| !k.is_empty())
        .unwrap_or(false);

    // Log incoming cookie presence for debugging
    let has_cookie_header = headers.get(axum::http::header::COOKIE).is_some();
    let cookie_preview = headers
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .map(|c| {
            // Show cookie names only, not values (security)
            let names: Vec<&str> = c
                .split(';')
                .map(|s| s.trim().split('=').next().unwrap_or("?"))
                .collect();
            names.join(",")
        })
        .unwrap_or_else(|| "none".to_string());

    debug!(target: "titen::auth", "SESSION_CHECK configured={} has_cookie_header={} cookies=[{}]", is_configured, has_cookie_header, cookie_preview);

    let authenticated = if !is_configured {
        true
    } else {
        let result = headers
            .get(axum::http::header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(|cookies| {
                cookies
                    .split(';')
                    .map(|c| c.trim())
                    .find(|c| c.starts_with("titen_session="))
                    .map(|c| c.trim_start_matches("titen_session=").to_string())
            })
            .map(|key| {
                subtle::ConstantTimeEq::ct_eq(
                    key.as_bytes(),
                    state.api_key.as_deref().unwrap_or_default().as_bytes(),
                )
                .into()
            })
            .unwrap_or(false);

        if !result {
            debug!(target: "titen::auth", "SESSION_FAIL no valid titen_session cookie or key mismatch");
        }

        result
    };

    info!(target: "titen::auth", "SESSION_RESULT authenticated={} requires_auth={}", authenticated, is_configured);

    Json(serde_json::json!({
        "requires_auth": is_configured,
        "authenticated": authenticated,
        "version": env!("CARGO_PKG_VERSION"),
        "timezone": titen_core::config::timezone(),
    }))
}

/// POST /api/auth/logout — clear session cookie
pub async fn logout() -> impl IntoResponse {
    let cookie = "titen_session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0";
    let mut headers = HeaderMap::new();
    // Safe: cookie is a hardcoded ASCII constant
    headers.insert(SET_COOKIE, HeaderValue::from_static(cookie));
    (
        StatusCode::OK,
        headers,
        Json(serde_json::json!({ "ok": true })),
    )
}
