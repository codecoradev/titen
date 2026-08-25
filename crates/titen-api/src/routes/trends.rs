//! Text-only trend detection over mentions/comments (issue #227).

use axum::{
    Json,
    extract::{Query, State},
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
) -> Json<TrendsResponse> {
    let window_minutes = q.window_minutes.unwrap_or(60).clamp(5, 1440);
    let windows = q.windows.unwrap_or(6).clamp(2, 24);
    let min_total = q.min_total.unwrap_or(2).max(1);
    let params = TrendParams {
        window_minutes,
        windows,
        min_total,
        ..Default::default()
    };

    // Pull up to window count × window duration of mentions
    let horizon_minutes = (windows as i64) * window_minutes;
    let mentions = state
        .store
        .list_mentions(&MentionFilter {
            account_id: q.account_id.clone(),
            limit: Some(1000),
            ..Default::default()
        })
        .await
        .unwrap_or_default();

    let now = chrono::Utc::now();
    let signals: Vec<TrendSignal> = mentions
        .into_iter()
        .filter_map(|m| {
            let text = m.text?;
            let at = m
                .mentioned_at
                .or(Some(m.fetched_at))
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|d| d.with_timezone(&chrono::Utc))?;
            // exclude signals older than the analysis horizon
            if now.signed_duration_since(at).num_minutes() > horizon_minutes {
                return None;
            }
            Some(TrendSignal { text, at })
        })
        .collect();

    let trends = analyze(&signals, &params).unwrap_or_default();
    Json(TrendsResponse {
        params: TrendsParamsOut {
            window_minutes,
            windows,
            min_total,
        },
        trends,
    })
}
