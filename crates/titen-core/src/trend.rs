//! Text-only topical trend detection & lifecycle signals.
//!
//! Inspired by Snapchat's LLM-enhanced trend detection (arXiv:2604.27131),
//! re-scoped for Titen's text-dominant sources (Threads mentions/comments):
//! deterministic term extraction + time-window velocity/acceleration analysis.
//! No model required; an optional LLM topic extractor can be toggled in config.

use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use utoipa::ToSchema;

/// A single text item with its timestamp, e.g. a mention or comment.
#[derive(Debug, Clone)]
pub struct TrendSignal {
    pub text: String,
    pub at: DateTime<Utc>,
}

/// Lifecycle stage of a tracked term.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum LifecycleStage {
    Emerging,
    Peaking,
    Fading,
    Stable,
}

/// Per-term trend analysis result.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TermTrend {
    pub term: String,
    /// Mentions per window, oldest → newest.
    pub counts: Vec<i64>,
    /// Average mentions per window.
    pub velocity: f64,
    /// Change of velocity between the latest two windows.
    pub acceleration: f64,
    pub stage: LifecycleStage,
}

/// Analysis parameters with sensible defaults.
#[derive(Debug, Clone)]
pub struct TrendParams {
    /// Window duration in minutes (default 60).
    pub window_minutes: i64,
    /// Number of windows to analyze (default 6).
    pub windows: usize,
    /// Minimum total mentions for a term to be reported (default 2).
    pub min_total: i64,
    /// Acceleration ratio threshold for emerging/fading (default 0.2).
    pub accel_threshold: f64,
}

impl Default for TrendParams {
    fn default() -> Self {
        Self {
            window_minutes: 60,
            windows: 6,
            min_total: 2,
            accel_threshold: 0.2,
        }
    }
}

/// Indonesian + English stopwords for term extraction.
const STOPWORDS: &[&str] = &[
    // English
    "the",
    "a",
    "an",
    "and",
    "or",
    "but",
    "if",
    "of",
    "to",
    "in",
    "on",
    "for",
    "with",
    "at",
    "by",
    "from",
    "is",
    "are",
    "was",
    "were",
    "be",
    "been",
    "this",
    "that",
    "these",
    "those",
    "it",
    "its",
    "as",
    "not",
    "no",
    "yes",
    "you",
    "your",
    "we",
    "our",
    "they",
    "them",
    "their",
    "i",
    "me",
    "my",
    "he",
    "she",
    "his",
    "her",
    "will",
    "would",
    "can",
    "could",
    "should",
    "just",
    "so",
    "than",
    "then",
    "there",
    "here",
    "what",
    "which",
    "who",
    "how",
    "when",
    "why",
    "all",
    "any",
    "some",
    "more",
    "most",
    "other",
    "such",
    "only",
    "own",
    "same",
    "too",
    "very",
    "s",
    "t",
    "don",
    "now",
    "do",
    "does",
    "did",
    "have",
    "has",
    "had",
    "get",
    "got",
    "out",
    "up",
    "about",
    "into",
    "over",
    "after",
    "before",
    "again",
    "further",
    "once",
    "because",
    "while",
    // Indonesian
    "yang",
    "dan",
    "atau",
    "tapi",
    "jika",
    "kalau",
    "untuk",
    "dengan",
    "pada",
    "di",
    "ke",
    "dari",
    "adalah",
    "ini",
    "itu",
    "tersebut",
    "tidak",
    "bukan",
    "iya",
    "kamu",
    "kau",
    "anda",
    "saya",
    "aku",
    "kami",
    "kita",
    "mereka",
    "dia",
    "beliau",
    "akan",
    "bisa",
    "dapat",
    "harus",
    "sudah",
    "telah",
    "sedang",
    "masih",
    "juga",
    "saja",
    "hanya",
    "ada",
    "punya",
    "lagi",
    "banget",
    "dong",
    "sih",
    "kok",
    "deh",
    "ya",
    "nah",
    "sangat",
    "gimana",
    "bagaimana",
    "kenapa",
    "mengapa",
    "kapan",
    "dimana",
    "di mana",
    "apa",
    "siapa",
    "semua",
    "setiap",
    "banyak",
    "sedikit",
    "lebih",
    "paling",
    "sama",
    "oleh",
    "dalam",
    "antar",
    "para",
    "sih",
];

/// Extract normalized terms (unigrams + bigrams) from text.
/// Deterministic, no model: lowercase, strip punctuation, drop stopwords & short tokens.
pub fn extract_terms(text: &str) -> Vec<String> {
    let tokens: Vec<String> = text
        .to_lowercase()
        .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
        .filter(|t| t.len() >= 3 && !t.starts_with('#') && !STOPWORDS.contains(t))
        .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .filter(|t| t.len() >= 3 && t.chars().any(|c| c.is_alphanumeric()))
        .collect();

    let mut terms: Vec<String> = tokens.clone();
    for pair in tokens.windows(2) {
        terms.push(format!("{} {}", pair[0], pair[1]));
    }
    terms
}

/// Assign each signal to a window index relative to `now`.
/// Window 0 is the oldest complete window; index is None if outside range.
fn window_index(
    at: DateTime<Utc>,
    now: DateTime<Utc>,
    window_minutes: i64,
    windows: usize,
) -> Option<usize> {
    // Guard against divide-by-zero when callers bypass the API-level clamp.
    if window_minutes <= 0 {
        return None;
    }
    let delta = now.signed_duration_since(at).num_minutes();
    if delta < 0 {
        return None; // future timestamps excluded
    }
    let idx = (delta / window_minutes) as usize;
    if idx < windows { Some(idx) } else { None }
}

/// Classify lifecycle stage from counts (oldest → newest window).
///
/// - `peaking`: latest window is the maximum
/// - `emerging`: velocity increasing and below the series max (early ramp)
/// - `fading`: velocity decreasing after a peak
/// - `stable`: flat within the acceleration threshold
fn classify_stage(
    counts: &[i64],
    velocity: f64,
    acceleration: f64,
    threshold: f64,
) -> LifecycleStage {
    if counts.len() < 2 {
        return LifecycleStage::Stable;
    }
    let last = *counts.last().unwrap() as f64;
    let max = *counts.iter().max().unwrap() as f64;

    // Acceleration relative to average velocity
    let rel_accel = if velocity > 0.0 {
        acceleration / velocity
    } else {
        0.0
    };

    // Fading: velocity clearly dropping after a peak
    if rel_accel <= -threshold {
        return LifecycleStage::Fading;
    }
    // Emerging: clear positive acceleration, still ramping up
    if rel_accel >= threshold && acceleration > 0.0 {
        return LifecycleStage::Emerging;
    }
    // Peaking: at/near the series max with flat (high but steady) volume
    if last >= max && max > 0.0 {
        return LifecycleStage::Peaking;
    }
    LifecycleStage::Stable
}

/// Analyze a set of text signals and return per-term trends, hottest first.
pub fn analyze(signals: &[TrendSignal], params: &TrendParams) -> Result<Vec<TermTrend>> {
    let now = Utc::now();
    let mut buckets: HashMap<String, Vec<i64>> = HashMap::new();

    for signal in signals {
        let Some(idx) = window_index(signal.at, now, params.window_minutes, params.windows) else {
            continue;
        };
        // Terms deduplicated per signal so one mention doesn't double-count a term.
        let unique: HashSet<String> = extract_terms(&signal.text).into_iter().collect();
        for term in unique {
            let counts = buckets
                .entry(term)
                .or_insert_with(|| vec![0; params.windows]);
            counts[params.windows - 1 - idx] += 1;
        }
    }

    let mut trends: Vec<TermTrend> = buckets
        .into_iter()
        .filter(|(_, counts)| counts.iter().sum::<i64>() >= params.min_total)
        .map(|(term, counts)| {
            let velocity = counts.iter().sum::<i64>() as f64 / counts.len() as f64;
            let n = counts.len();
            let prev_avg = counts[..n - 1].iter().sum::<i64>() as f64 / (n - 1) as f64;
            let last = *counts.last().unwrap() as f64;
            let acceleration = last - prev_avg;
            let stage = classify_stage(&counts, velocity, acceleration, params.accel_threshold);
            TermTrend {
                term,
                counts,
                velocity,
                acceleration,
                stage,
            }
        })
        .collect();

    // Hottest first: acceleration desc, then total volume desc
    trends.sort_by(|a, b| {
        b.acceleration
            .partial_cmp(&a.acceleration)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(
                b.counts
                    .iter()
                    .sum::<i64>()
                    .cmp(&a.counts.iter().sum::<i64>()),
            )
    });
    Ok(trends)
}

/// Optional LLM-based topic extractor (toggle in config; off by default).
/// Implementations may replace/augment deterministic `extract_terms`.
#[allow(async_fn_in_trait)]
pub trait TopicExtractor {
    async fn extract(&self, text: &str) -> Result<Vec<String>>;
}

/// Deterministic extractor used when the LLM toggle is off.
pub struct DeterministicExtractor;

impl TopicExtractor for DeterministicExtractor {
    async fn extract(&self, text: &str) -> Result<Vec<String>> {
        Ok(extract_terms(text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn sig(minutes_ago: i64, text: &str) -> TrendSignal {
        TrendSignal {
            text: text.to_string(),
            at: Utc::now() - Duration::minutes(minutes_ago),
        }
    }

    #[test]
    fn extract_terms_filters_stopwords_and_short() {
        let terms = extract_terms("The Rust compiler yang sangat cepat!");
        assert!(terms.contains(&"rust".to_string()));
        assert!(terms.contains(&"compiler".to_string()));
        assert!(terms.contains(&"cepat".to_string()));
        assert!(
            !terms
                .iter()
                .any(|t| t == "the" || t == "yang" || t == "sangat")
        );
        assert!(!terms.iter().any(|t| t == "di"));
    }

    #[test]
    fn extract_terms_bigram() {
        let terms = extract_terms("webassembly performance");
        assert!(terms.contains(&"webassembly performance".to_string()));
    }

    #[test]
    fn analyze_emerging() {
        let p = TrendParams {
            window_minutes: 60,
            windows: 4,
            min_total: 2,
            accel_threshold: 0.2,
        };
        let signals = vec![
            sig(200, "rust release"),
            sig(100, "rust release notes"),
            sig(30, "rust release party"),
            sig(20, "rust release"),
        ];
        let trends = analyze(&signals, &p).unwrap();
        let rust = trends
            .iter()
            .find(|t| t.term.contains("rust release"))
            .unwrap();
        assert_eq!(rust.stage, LifecycleStage::Emerging);
        assert!(rust.acceleration > 0.0);
    }

    #[test]
    fn analyze_fading() {
        let p = TrendParams {
            window_minutes: 60,
            windows: 4,
            min_total: 2,
            accel_threshold: 0.2,
        };
        // 3 mentions in window 1 (~110 min ago), nothing since → counts [0,3,0,0]
        let signals = vec![
            sig(115, "old drama"),
            sig(110, "old drama"),
            sig(105, "old drama"),
        ];
        let trends = analyze(&signals, &p).unwrap();
        let drama = trends.iter().find(|t| t.term == "old drama").unwrap();
        assert_eq!(drama.stage, LifecycleStage::Fading);
    }

    #[test]
    fn analyze_peaking() {
        let p = TrendParams {
            window_minutes: 60,
            windows: 4,
            min_total: 2,
            accel_threshold: 0.2,
        };
        // 2 mentions per window, steady → counts [2,2,2,2], flat at max
        let signals = vec![
            sig(200, "launch day"),
            sig(190, "launch day"),
            sig(130, "launch day"),
            sig(120, "launch day"),
            sig(70, "launch day"),
            sig(60, "launch day"),
            sig(20, "launch day"),
            sig(10, "launch day"),
        ];
        let trends = analyze(&signals, &p).unwrap();
        let launch = trends.iter().find(|t| t.term == "launch day").unwrap();
        assert_eq!(launch.stage, LifecycleStage::Peaking);
    }

    #[test]
    fn analyze_min_total_filters_noise() {
        let p = TrendParams {
            window_minutes: 60,
            windows: 4,
            min_total: 3,
            accel_threshold: 0.2,
        };
        let signals = vec![sig(30, "singleton mention"), sig(200, "another thing")];
        let trends = analyze(&signals, &p).unwrap();
        assert!(trends.is_empty());
    }

    #[test]
    fn analyze_ignores_out_of_range_and_future() {
        let p = TrendParams {
            window_minutes: 60,
            windows: 2,
            min_total: 1,
            accel_threshold: 0.2,
        };
        let signals = vec![
            sig(30, "recent rust"),
            sig(500, "very old rust"),
            sig(-10, "future rust"),
        ];
        let trends = analyze(&signals, &p).unwrap();
        let rust = trends.iter().find(|t| t.term == "recent rust").unwrap();
        // only the in-range signal counts
        assert_eq!(rust.counts.iter().sum::<i64>(), 1);
    }

    #[tokio::test]
    async fn deterministic_extractor_matches_extract_terms() {
        let e = DeterministicExtractor;
        let terms = e.extract("rust release notes").await.unwrap();
        assert_eq!(terms, extract_terms("rust release notes"));
    }
}
