use axum::{
    Json,
    extract::State,
    http::{StatusCode, header::SET_COOKIE, HeaderMap, HeaderValue},
    response::IntoResponse,
};
use serde::Deserialize;

use crate::server::{AppState, error_response};

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
            headers.insert(SET_COOKIE, HeaderValue::from_str(cookie).unwrap());
            return (StatusCode::OK, headers, Json(LoginResponse { valid: true })).into_response();
        }
    };

    if subtle::ConstantTimeEq::ct_eq(input.api_key.as_bytes(), required_key.as_bytes()).into() {
        // Set httpOnly cookie with the API key
        let cookie_value = format!(
            "titen_session={}; Path=/; HttpOnly; SameSite=Strict; Max-Age=604800",
            input.api_key
        );
        let mut headers = HeaderMap::new();
        headers.insert(SET_COOKIE, HeaderValue::from_str(&cookie_value).unwrap());
        (StatusCode::OK, headers, Json(LoginResponse { valid: true })).into_response()
    } else {
        let (status, body) =
            error_response(StatusCode::UNAUTHORIZED, "INVALID_API_KEY", "Invalid API key");
        (status, HeaderMap::new(), Json(body)).into_response()
    }
}

/// GET /api/auth/session — check if current session is valid
pub async fn session(State(state): State<AppState>) -> impl IntoResponse {
    let is_configured = state
        .api_key
        .as_ref()
        .map(|k| !k.is_empty())
        .unwrap_or(false);

    Json(serde_json::json!({
        "requires_auth": is_configured,
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// POST /api/auth/logout — clear session cookie
pub async fn logout() -> impl IntoResponse {
    let cookie = "titen_session=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0";
    let mut headers = HeaderMap::new();
    headers.insert(SET_COOKIE, HeaderValue::from_str(cookie).unwrap());
    (StatusCode::OK, headers, Json(serde_json::json!({ "ok": true })))
}
