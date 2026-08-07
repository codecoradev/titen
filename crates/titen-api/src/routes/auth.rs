use axum::{
    Json,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode, header::SET_COOKIE},
    response::IntoResponse,
};
use serde::Deserialize;

use crate::server::{AppState, ErrorResponse};

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
/// 2. SameSite=Strict prevents CSRF
/// 3. The API key is already a bearer-style secret
pub async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginRequest>,
) -> impl IntoResponse {
    let required_key = match &state.api_key {
        Some(key) if !key.is_empty() => key,
        _ => {
            // Dev mode — no API key configured, accept anything
            let cookie = "titen_session=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0";
            let mut headers = HeaderMap::new();
            // Safe: cookie is a hardcoded ASCII constant
            headers.insert(SET_COOKIE, HeaderValue::from_static(cookie));
            return (StatusCode::OK, headers, Json(LoginResponse { valid: true })).into_response();
        }
    };

    if subtle::ConstantTimeEq::ct_eq(input.api_key.as_bytes(), required_key.as_bytes()).into() {
        // Set httpOnly cookie with the API key.
        // Secure flag added only when TITEN_COOKIE_SECURE=true (production HTTPS).
        // In dev (HTTP), the Secure attribute must be omitted entirely, not set to false.
        let secure = std::env::var("TITEN_COOKIE_SECURE")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);
        let secure_attr = if secure { "; Secure" } else { "" };
        let cookie_value = format!(
            "titen_session={}; Path=/; HttpOnly; SameSite=Strict; Max-Age=604800{secure_attr}",
            input.api_key
        );
        let mut headers = HeaderMap::new();
        // Safe unwrap: cookie_value contains only the API key (validated ASCII/UTF-8 by ct_eq
        // against the configured key) plus standard cookie syntax characters.
        match HeaderValue::from_str(&cookie_value) {
            Ok(val) => headers.insert(SET_COOKIE, val),
            Err(_) => {
                // Non-ASCII API key — reject rather than panic
                let body = ErrorResponse {
                    error: "API key contains invalid characters for cookie storage".to_string(),
                    code: "INVALID_API_KEY".to_string(),
                };
                return (StatusCode::BAD_REQUEST, HeaderMap::new(), Json(body)).into_response();
            }
        };
        (StatusCode::OK, headers, Json(LoginResponse { valid: true })).into_response()
    } else {
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

    // Check if request has a valid session cookie
    let authenticated = if !is_configured {
        // Dev mode — no API key, always authenticated
        true
    } else {
        // Extract titen_session cookie and compare against the configured API key
        headers
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
            .unwrap_or(false)
    };

    Json(serde_json::json!({
        "requires_auth": is_configured,
        "authenticated": authenticated,
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// POST /api/auth/logout — clear session cookie
pub async fn logout() -> impl IntoResponse {
    let cookie = "titen_session=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0";
    let mut headers = HeaderMap::new();
    // Safe: cookie is a hardcoded ASCII constant
    headers.insert(SET_COOKIE, HeaderValue::from_static(cookie));
    (
        StatusCode::OK,
        headers,
        Json(serde_json::json!({ "ok": true })),
    )
}
