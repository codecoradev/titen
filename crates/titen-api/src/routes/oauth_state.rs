// #237 — OAuth state parameter (CSRF protection)
//
// The Threads OAuth flow previously sent no `state` parameter, so the
// `/auth/callback` page could be driven by an attacker-crafted callback URL
// (login CSRF). This module adds a one-time state token:
//
//   1. `POST /api/oauth/state` (authenticated) — generates a 256-bit random
//      token, stores it bound to the caller's API key / session, and returns
//      `{ state }`. TTL: 10 minutes, single use.
//   2. `POST /api/oauth/exchange` (authenticated) — now requires `state` and
//      validates it: must exist, belong to the caller, be unexpired, and is
//      atomically deleted on success (replay impossible).

use axum::{Json, extract::State, http::HeaderMap, http::StatusCode};
use getrandom::fill as getrandom_fill;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

use crate::server::AppState;

/// OAuth state tokens live 10 minutes — enough to complete a login redirect,
/// short enough that stale tokens expire quickly.
pub const STATE_TTL_SECS: i64 = 600;

fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Resolve the caller identity for state binding from request headers,
/// mirroring `api_key_auth`: raw `X-API-Key` header first, then the session
/// cookie resolved through the session store. Returns `None` when neither is
/// present (the auth middleware would have rejected the request anyway; dev
/// mode with no configured key yields `Some("dev")` at the call site).
pub async fn caller_key_from_headers(_state: &AppState, headers: &HeaderMap) -> Option<String> {
    if let Some(k) = headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
    {
        return Some(k);
    }
    let cookie_token = headers
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies
                .split(';')
                .map(|c| c.trim())
                .find(|c| c.starts_with("titen_session="))
                .map(|c| c.trim_start_matches("titen_session=").to_string())
        });
    match cookie_token {
        Some(ref token) => crate::routes::auth::validate_session(token).await,
        None => None,
    }
}

/// Issue a one-time OAuth state token bound to the caller.
pub async fn create_state(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> (StatusCode, Json<serde_json::Value>) {
    let required_key = state.api_key.clone().unwrap_or_default();
    let caller = if required_key.is_empty() {
        // Dev mode (no key configured): auth middleware lets everyone through.
        "dev".to_string()
    } else {
        match caller_key_from_headers(&state, &headers).await {
            Some(k) => k,
            None => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({
                        "error": "Invalid or missing API key",
                        "code": "UNAUTHORIZED"
                    })),
                );
            }
        }
    };

    let mut buf = [0u8; 32]; // 256 bits of entropy
    if let Err(e) = getrandom_fill(&mut buf) {
        warn!(target: "titen::oauth", "OAUTH_STATE_FAIL rng: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to generate OAuth state token.",
                "code": "STATE_GENERATION_FAILED"
            })),
        );
    }
    let state_value = hex::encode(buf);

    let expires_at = now_epoch() + STATE_TTL_SECS;
    if let Err(e) = state
        .store
        .insert_oauth_state(&state_value, &caller, expires_at)
        .await
    {
        warn!(target: "titen::oauth", "OAUTH_STATE_FAIL store: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to persist OAuth state token.",
                "code": "STATE_STORE_FAILED"
            })),
        );
    }

    // Opportunistic cleanup of expired states (cheap: indexed column).
    let _ = state.store.prune_expired_oauth_states().await;

    info!(target: "titen::oauth", "OAUTH_STATE_ISSUED");

    (
        StatusCode::OK,
        Json(serde_json::json!({ "state": state_value })),
    )
}

/// Consume a state token: validates owner + expiry and deletes it
/// atomically (single use). Returns Ok(()) when valid.
pub async fn consume_state(
    state: &AppState,
    caller: &str,
    token: &str,
) -> Result<(), (StatusCode, serde_json::Value)> {
    let deleted = state
        .store
        .consume_oauth_state(token, caller)
        .await
        .map_err(|e| {
            warn!(target: "titen::oauth", "OAUTH_STATE_VALIDATE_FAIL store: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({
                    "error": "Failed to validate OAuth state token.",
                    "code": "STATE_STORE_FAILED"
                }),
            )
        })?;
    if deleted {
        Ok(())
    } else {
        warn!(target: "titen::oauth", "OAUTH_STATE_REJECTED invalid/missing/expired state");
        Err((
            StatusCode::BAD_REQUEST,
            serde_json::json!({
                "error": "Invalid, expired, or already-used OAuth state token. Restart the connection flow.",
                "code": "INVALID_OAUTH_STATE"
            }),
        ))
    }
}

/// Validates AND consumes the `state` on an exchange request, atomically.
/// Used for the legacy `{code, app_id, app_secret, redirect_uri}` clients:
/// they cannot know the stored binding, so any state that is unexpired and
/// not yet consumed is accepted exactly once — the DELETE is the authority,
/// so concurrent exchanges race on it and exactly one caller wins. Still
/// unguessable at 256-bit. Returns the error response to send on failure.
pub async fn validate_state_shallow(
    state: &AppState,
    token: &str,
) -> Result<(), (StatusCode, serde_json::Value)> {
    let consumed = state
        .store
        .consume_any_oauth_state(token)
        .await
        .map_err(|e| {
            warn!(target: "titen::oauth", "OAUTH_STATE_VALIDATE_FAIL store: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({
                    "error": "Failed to validate OAuth state token.",
                    "code": "STATE_STORE_FAILED"
                }),
            )
        })?;
    if consumed {
        Ok(())
    } else {
        warn!(target: "titen::oauth", "OAUTH_STATE_REJECTED legacy invalid/missing/expired state");
        Err((
            StatusCode::BAD_REQUEST,
            serde_json::json!({
                "error": "Invalid, expired, or already-used OAuth state token. Restart the connection flow.",
                "code": "INVALID_OAUTH_STATE"
            }),
        ))
    }
}
