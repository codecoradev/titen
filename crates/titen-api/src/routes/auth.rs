use axum::{
    Json,
    extract::{ConnectInfo, State},
    http::{HeaderMap, HeaderValue, StatusCode, header::SET_COOKIE},
    response::IntoResponse,
};
use dashmap::DashMap;
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::LazyLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

use crate::server::{AppState, ErrorResponse};

/// Simple in-memory rate limiter for login attempts.
/// Tracks failed attempts per IP, blocks after MAX_ATTEMPTS within WINDOW.
/// Uses DashMap for lock-free concurrent access (no thread starvation under load).
const MAX_LOGIN_ATTEMPTS: u32 = 5;
const LOGIN_WINDOW: Duration = Duration::from_secs(60); // 1 minute
const LOGIN_LOCKOUT: Duration = Duration::from_secs(300); // 5 minutes
const MAX_TRACKED_IPS: usize = 10_000; // prevent unbounded memory growth

static LOGIN_ATTEMPTS: LazyLock<DashMap<String, Vec<Instant>>> = LazyLock::new(DashMap::new);

/// P5.4: Opaque session token store — maps session token → API key.
/// Instead of storing the raw API key in the cookie, we issue an opaque
/// random token that maps back to the key server-side. This decouples the
/// cookie value from the actual secret, enabling rotation and revocation.
static SESSIONS: LazyLock<DashMap<String, SessionEntry>> = LazyLock::new(DashMap::new);

const SESSION_TTL: Duration = Duration::from_secs(604800); // 7 days
const MAX_SESSIONS: usize = 10_000;

struct SessionEntry {
    api_key: String,
    expires_at: u64, // unix epoch seconds
}

/// Generate a cryptographically random opaque token (256-bit entropy).
/// Returns Result so callers can handle RNG failures gracefully.
fn generate_session_token() -> Result<String, getrandom::Error> {
    let mut buf = [0u8; 32]; // 256 bits of entropy
    getrandom::fill(&mut buf)?;

    // Hex-encode for cookie-safe representation (64 chars, alphanumeric only)
    Ok(hex::encode(buf))
}

/// Issue a session token for a given API key. Returns the token string,
/// or None if the system RNG fails (catastrophic — should never happen).
fn issue_session(api_key: &str) -> Option<String> {
    let token = generate_session_token().ok()?;
    let expires_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + SESSION_TTL.as_secs();

    // Periodic cleanup: prune expired sessions
    if SESSIONS.len() > MAX_SESSIONS {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        SESSIONS.retain(|_, entry| entry.expires_at > now);
    }

    SESSIONS.insert(
        token.clone(),
        SessionEntry {
            api_key: api_key.to_string(),
            expires_at,
        },
    );

    Some(token)
}

/// Validate a session token. Returns the associated API key if valid and not expired.
pub fn validate_session(token: &str) -> Option<String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    SESSIONS.get(token).and_then(|entry| {
        if entry.expires_at > now {
            Some(entry.api_key.clone())
        } else {
            None
        }
    })
}

/// Check if an IP is rate-limited. Returns true if the IP should be blocked.
fn is_rate_limited(ip: &str) -> bool {
    let now = Instant::now();

    match LOGIN_ATTEMPTS.get_mut(ip) {
        Some(mut timestamps) => {
            // Prune entries older than the lockout window
            timestamps.retain(|t| now.duration_since(*t) < LOGIN_LOCKOUT);

            // Count attempts within the sliding window
            let recent = timestamps
                .iter()
                .filter(|t| now.duration_since(**t) < LOGIN_WINDOW)
                .count();

            timestamps.len() >= MAX_LOGIN_ATTEMPTS as usize || recent >= MAX_LOGIN_ATTEMPTS as usize
        }
        None => false,
    }
}

/// Record a failed login attempt for an IP.
fn record_failed_attempt(ip: &str) {
    // Periodic cleanup: remove IPs with no recent attempts to prevent unbounded growth
    if LOGIN_ATTEMPTS.len() > MAX_TRACKED_IPS {
        let now = Instant::now();
        LOGIN_ATTEMPTS.retain(|_, ts| ts.iter().any(|t| now.duration_since(*t) < LOGIN_LOCKOUT));
    }

    LOGIN_ATTEMPTS
        .entry(ip.to_string())
        .or_default()
        .push(Instant::now());
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
/// P5.4: The cookie stores an opaque session token, NOT the raw API key.
/// The token maps to the key server-side via the SESSIONS DashMap.
/// This decouples the cookie value from the actual secret, enabling:
/// 1. Session rotation on re-login
/// 2. Session revocation without changing the API key
/// 3. No raw secret exposure in cookie payloads
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

        // P5.4: Issue opaque session token instead of raw API key
        let session_token = match issue_session(&input.api_key) {
            Some(t) => t,
            None => {
                warn!(target: "titen::auth", "LOGIN_REJECT session token generation failed (RNG failure)");
                let body = ErrorResponse {
                    error: "Failed to create session".to_string(),
                    code: "SESSION_ERROR".to_string(),
                };
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    HeaderMap::new(),
                    Json(body),
                )
                    .into_response();
            }
        };
        let cookie_value = format!(
            "titen_session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=604800{secure_attr}",
            session_token
        );
        let mut resp_headers = HeaderMap::new();
        match HeaderValue::from_str(&cookie_value) {
            Ok(val) => resp_headers.insert(SET_COOKIE, val),
            Err(_) => {
                warn!(target: "titen::auth", "LOGIN_REJECT invalid chars in session token");
                let body = ErrorResponse {
                    error: "Failed to create session".to_string(),
                    code: "SESSION_ERROR".to_string(),
                };
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    HeaderMap::new(),
                    Json(body),
                )
                    .into_response();
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
        // P5.4: Validate opaque session token — if validate_session succeeds,
        // the session is independently valid. No need to re-compare the key
        // against state.api_key; the session was issued after key verification.
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
            .map(|token| validate_session(&token).is_some())
            .unwrap_or(false);

        if !result {
            debug!(target: "titen::auth", "SESSION_FAIL no valid titen_session cookie or session expired");
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
