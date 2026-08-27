//! Text-only trend detection over mentions/comments (issue #227).

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use titen_core::models::MentionFilter;
use titen_core::trend::{TermTrend, TrendParams, TrendSignal, analyze};

use crate::server::AppState;

#[derive(Deserialize, Debug)]
pub struct TrendsQuery {
    pub account_id: Option<String>,
    /// Window duration in minutes (default 60).
    pub window_minutes: Option<i64>,
    /// Number of windows to analyze (default 6, max 24).
    pub windows: Option<usize>,
    /// Minimum total mentions for a term to be reported (default 2).
    pub min_total: Option<i64>,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct TrendsResponse {
    pub params: TrendsParamsOut,
    pub trends: Vec<TermTrend>,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct TrendsParamsOut {
    pub window_minutes: i64,
    pub windows: usize,
    pub min_total: i64,
}

#[utoipa::path(
    get,
    path = "/api/insights/trends",
    tag = "insights",
    params(
        ("account_id" = Option<String>, Query, description = "Filter mentions by account"),
        ("window_minutes" = Option<i64>, Query, description = "Window duration in minutes (default 60)"),
        ("windows" = Option<usize>, Query, description = "Number of windows (default 6, max 24)"),
        ("min_total" = Option<i64>, Query, description = "Min total mentions per term (default 2)"),
    ),
    responses(
        (status = 200, description = "Per-term trend lifecycle (emerging/peaking/fading/stable)", body = TrendsResponse),
    ),
    security(("api_key" = [])),
)]
/// Text-only topical trend detection: deterministic term extraction over
/// mentions + velocity/acceleration lifecycle classification.
/// No LLM involved by default (LLM topic extraction is an optional future toggle).
pub async fn get_trends(
    State(state): State<AppState>,
    Query(q): Query<TrendsQuery>,
) -> Result<Json<TrendsResponse>, (StatusCode, Json<serde_json::Value>)> {
    let window_minutes = q.window_minutes.unwrap_or(60).clamp(5, 1440);
    let windows = q.windows.unwrap_or(6).clamp(2, 24);
    let min_total = q.min_total.unwrap_or(2).max(1);
    // Cap the analysis horizon at 24h so unbounded=true cannot load weeks of
    // mentions into memory. The requested windows are shrunk to fit the cap
    // so the echoed params, the DB fetch, and the analysis stay consistent.
    const MAX_HORIZON_MINUTES: i64 = 24 * 60;
    let mut windows = windows;
    while windows > 2 && (windows as i64) * window_minutes > MAX_HORIZON_MINUTES {
        windows -= 1;
    }
    let horizon_minutes = (windows as i64) * window_minutes;
    let params = TrendParams {
        window_minutes,
        windows,
        min_total,
        ..Default::default()
    };

    // Horizon filter is pushed into the SQL query (date_from on fetched_at)
    // so short windows do not fetch irrelevant rows and long windows are not
    // silently truncated by a fixed limit.
    let now = chrono::Utc::now();
    let date_from = (now - chrono::Duration::minutes(horizon_minutes)).to_rfc3339();
    let mentions = state
        .store
        .list_mentions(&MentionFilter {
            account_id: q.account_id.clone(),
            date_from: Some(date_from),
            limit: None, // unbounded: horizon already constrains via date_from
            unbounded: true,
            ..Default::default()
        })
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "trends: failed to list mentions");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to load mentions",
                    "code": "INTERNAL"
                })),
            )
        })?;

    let signals: Vec<TrendSignal> = mentions
        .into_iter()
        .filter_map(|m| {
            let text = m.text?;
            // Tolerant parse: legacy rows may hold `+0000` or space-format
            // timestamps that strict RFC3339 parsing silently drops.
            let ts: Option<String> = m.mentioned_at.or(Some(m.fetched_at));
            let at = ts.and_then(|s| titen_core::time::parse_utc(&s))?;
            // exclude signals older than the analysis horizon
            if now.signed_duration_since(at).num_minutes() > horizon_minutes {
                return None;
            }
            Some(TrendSignal { text, at })
        })
        .collect();

    let trends = analyze(&signals, &params).unwrap_or_default();
    Ok(Json(TrendsResponse {
        params: TrendsParamsOut {
            window_minutes,
            windows,
            min_total,
        },
        trends,
    }))
}
