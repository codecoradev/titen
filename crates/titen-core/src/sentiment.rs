use crate::error::Result;
use crate::models::{SentimentResult, SentimentSummary};

/// Trait for sentiment analysis engines
#[async_trait::async_trait]
pub trait SentimentEngine: Send + Sync {
    /// Analyze a single text and return sentiment result
    async fn analyze(&self, text: &str) -> Result<SentimentResult>;

    /// Analyze a batch of texts
    async fn analyze_batch(&self, texts: &[&str]) -> Result<Vec<SentimentResult>> {
        // Default: sequential calls
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.analyze(text).await?);
        }
        Ok(results)
    }
}

/// Stub sentiment engine — always returns neutral/0.0 for development/testing
pub struct StubEngine;

#[async_trait::async_trait]
impl SentimentEngine for StubEngine {
    async fn analyze(&self, _text: &str) -> Result<SentimentResult> {
        Ok(SentimentResult {
            label: "neutral".to_string(),
            score: 0.0,
        })
    }
}

/// Keyword-based sentiment engine — simple rule-based analysis
/// Uses basic positive/negative word matching for Indonesian and English
pub struct KeywordEngine {
    positive: Vec<String>,
    negative: Vec<String>,
}

impl Default for KeywordEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl KeywordEngine {
    pub fn new() -> Self {
        Self {
            positive: vec![
                // English
                "good".into(),
                "great".into(),
                "love".into(),
                "amazing".into(),
                "awesome".into(),
                "excellent".into(),
                "perfect".into(),
                "nice".into(),
                "best".into(),
                "wonderful".into(),
                "thanks".into(),
                "thank".into(),
                "helpful".into(),
                "cool".into(),
                "happy".into(),
                "beautiful".into(),
                "fantastic".into(),
                "brilliant".into(),
                "superb".into(),
                // Indonesian
                "bagus".into(),
                "keren".into(),
                "mantap".into(),
                "luar biasa".into(),
                "terima kasih".into(),
                "makasih".into(),
                "hebat".into(),
                "jos".into(),
                "top".into(),
                "sip".into(),
                "mantul".into(),
                "keren".into(),
                "enak".into(),
                "suka".into(),
                "senang".into(),
                "baik".into(),
            ],
            negative: vec![
                // English
                "bad".into(),
                "terrible".into(),
                "awful".into(),
                "worst".into(),
                "hate".into(),
                "poor".into(),
                "disappointing".into(),
                "boring".into(),
                "ugly".into(),
                "broken".into(),
                "useless".into(),
                "waste".into(),
                "fail".into(),
                "failed".into(),
                "sucks".into(),
                "trash".into(),
                "horrible".into(),
                "annoying".into(),
                "frustrating".into(),
                // Indonesian
                "jelek".into(),
                "buruk".into(),
                "gagal".into(),
                "sampah".into(),
                "nyebelin".into(),
                "ribet".into(),
                "susah".into(),
                "error".into(),
                "bug".into(),
                "lemot".into(),
                "parah".into(),
                "nggak".into(),
            ],
        }
    }

    fn score_text(&self, text: &str) -> (String, f64) {
        let lower = text.to_lowercase();
        let mut score = 0i32;
        let mut matches = 0i32;

        for word in &self.positive {
            if lower.contains(word) {
                score += 1;
                matches += 1;
            }
        }
        for word in &self.negative {
            if lower.contains(word) {
                score -= 1;
                matches += 1;
            }
        }

        if matches == 0 {
            return ("neutral".to_string(), 0.0);
        }

        let normalized = score as f64 / matches as f64; // -1.0 to 1.0
        let label = if normalized > 0.1 {
            "positive"
        } else if normalized < -0.1 {
            "negative"
        } else {
            "neutral"
        };

        (label.to_string(), normalized)
    }
}

#[async_trait::async_trait]
impl SentimentEngine for KeywordEngine {
    async fn analyze(&self, text: &str) -> Result<SentimentResult> {
        let (label, score) = self.score_text(text);
        Ok(SentimentResult { label, score })
    }

    async fn analyze_batch(&self, texts: &[&str]) -> Result<Vec<SentimentResult>> {
        // Keyword analysis is CPU-bound, no need for async
        Ok(texts
            .iter()
            .map(|t| {
                let (label, score) = self.score_text(t);
                SentimentResult { label, score }
            })
            .collect())
    }
}

/// Build a sentiment engine based on ENV config
pub fn build_engine(engine_type: &str) -> Box<dyn SentimentEngine> {
    match engine_type {
        "keyword" => Box::new(KeywordEngine::new()),
        _ => Box::new(StubEngine), // default: stub
    }
}

/// Compute sentiment summary from a list of results
pub fn compute_summary(results: &[SentimentResult]) -> SentimentSummary {
    let total = results.len() as i64;
    let mut positive = 0i64;
    let mut negative = 0i64;
    let mut neutral = 0i64;
    let mut sum = 0.0f64;

    for r in results {
        match r.label.as_str() {
            "positive" => positive += 1,
            "negative" => negative += 1,
            _ => neutral += 1,
        }
        sum += r.score;
    }

    let average_score = if total > 0 { sum / total as f64 } else { 0.0 };

    SentimentSummary {
        total,
        positive,
        negative,
        neutral,
        average_score,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::SentimentResult;

    // ─── StubEngine ────────────────────────────────────────

    #[tokio::test]
    async fn stub_engine_always_returns_neutral() {
        let engine = StubEngine;
        let result = engine.analyze("any text at all").await.unwrap();
        assert_eq!(result.label, "neutral");
        assert_eq!(result.score, 0.0);
    }

    #[tokio::test]
    async fn stub_engine_neutral_regardless_of_input() {
        let engine = StubEngine;
        for text in [
            "I love this amazing product",
            "This is terrible and awful",
            "",
            "bagus mantap luar biasa",
            "jelek parah buruk sampah",
        ] {
            let result = engine.analyze(text).await.unwrap();
            assert_eq!(
                result.label, "neutral",
                "StubEngine should always be neutral for: {text}"
            );
            assert_eq!(result.score, 0.0);
        }
    }

    #[tokio::test]
    async fn stub_engine_analyze_batch_returns_all_neutral() {
        let engine = StubEngine;
        let results = engine
            .analyze_batch(&["great", "terrible", "neutral text"])
            .await
            .unwrap();
        assert_eq!(results.len(), 3);
        for r in &results {
            assert_eq!(r.label, "neutral");
            assert_eq!(r.score, 0.0);
        }
    }

    // ─── KeywordEngine: English ────────────────────────────

    #[tokio::test]
    async fn keyword_positive_english() {
        let engine = KeywordEngine::new();
        let result = engine.analyze("This is great and amazing").await.unwrap();
        assert_eq!(result.label, "positive");
        assert!(result.score > 0.0);
    }

    #[tokio::test]
    async fn keyword_negative_english() {
        let engine = KeywordEngine::new();
        let result = engine.analyze("This is terrible and awful").await.unwrap();
        assert_eq!(result.label, "negative");
        assert!(result.score < 0.0);
    }

    #[tokio::test]
    async fn keyword_neutral_english() {
        let engine = KeywordEngine::new();
        let result = engine.analyze("The sky is blue today").await.unwrap();
        assert_eq!(result.label, "neutral");
        assert_eq!(result.score, 0.0);
    }

    // ─── KeywordEngine: Indonesian ────────────────────────

    #[tokio::test]
    async fn keyword_positive_indonesian() {
        let engine = KeywordEngine::new();

        let result = engine.analyze("Produknya bagus sekali").await.unwrap();
        assert_eq!(result.label, "positive");

        let result = engine.analyze("Mantap banget").await.unwrap();
        assert_eq!(result.label, "positive");

        // "bagus" (+1) + "jelek" (-1) = 0 → neutral (balanced score)
        let result = engine.analyze("Bagus jelek sih").await.unwrap();
        assert_eq!(result.label, "neutral");
    }

    #[tokio::test]
    async fn keyword_negative_indonesian() {
        let engine = KeywordEngine::new();

        let result = engine.analyze("Produknya jelek").await.unwrap();
        assert_eq!(result.label, "negative");

        let result = engine.analyze("Parah banget ini").await.unwrap();
        assert_eq!(result.label, "negative");
    }

    #[tokio::test]
    async fn keyword_neutral_indonesian() {
        let engine = KeywordEngine::new();
        let result = engine.analyze("Hari ini cerah").await.unwrap();
        assert_eq!(result.label, "neutral");
        assert_eq!(result.score, 0.0);
    }

    // ─── KeywordEngine: batch ─────────────────────────────

    #[tokio::test]
    async fn keyword_batch_mixed() {
        let engine = KeywordEngine::new();
        let results = engine
            .analyze_batch(&["great product", "terrible quality", "just okay"])
            .await
            .unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].label, "positive");
        assert_eq!(results[1].label, "negative");
        assert_eq!(results[2].label, "neutral");
    }

    // ─── KeywordEngine: case insensitivity ─────────────────

    #[tokio::test]
    async fn keyword_case_insensitive() {
        let engine = KeywordEngine::new();
        let lower = engine.analyze("this is GOOD").await.unwrap();
        let upper = engine.analyze("this is good").await.unwrap();
        assert_eq!(lower.label, upper.label);
        assert_eq!(lower.score, upper.score);
    }

    // ─── KeywordEngine: multi-word phrases ────────────────

    #[tokio::test]
    async fn keyword_multi_word_phrases() {
        let engine = KeywordEngine::new();

        // "terima kasih" is a positive multi-word phrase
        let result = engine.analyze("Terima kasih banyak").await.unwrap();
        assert_eq!(result.label, "positive");

        // "luar biasa" is a positive multi-word phrase
        let result = engine.analyze("Film ini luar biasa").await.unwrap();
        assert_eq!(result.label, "positive");
    }

    // ─── compute_summary ───────────────────────────────────

    #[test]
    fn compute_summary_empty() {
        let summary = compute_summary(&[]);
        assert_eq!(summary.total, 0);
        assert_eq!(summary.positive, 0);
        assert_eq!(summary.negative, 0);
        assert_eq!(summary.neutral, 0);
        assert_eq!(summary.average_score, 0.0);
    }

    #[test]
    fn compute_summary_all_positive() {
        let results = vec![
            SentimentResult {
                label: "positive".into(),
                score: 0.5,
            },
            SentimentResult {
                label: "positive".into(),
                score: 0.8,
            },
            SentimentResult {
                label: "positive".into(),
                score: 1.0,
            },
        ];
        let summary = compute_summary(&results);
        assert_eq!(summary.total, 3);
        assert_eq!(summary.positive, 3);
        assert_eq!(summary.negative, 0);
        assert_eq!(summary.neutral, 0);
        assert!((summary.average_score - (0.5 + 0.8 + 1.0) / 3.0).abs() < 1e-10);
    }

    #[test]
    fn compute_summary_mixed() {
        let results = vec![
            SentimentResult {
                label: "positive".into(),
                score: 1.0,
            },
            SentimentResult {
                label: "negative".into(),
                score: -1.0,
            },
            SentimentResult {
                label: "neutral".into(),
                score: 0.0,
            },
            SentimentResult {
                label: "positive".into(),
                score: 0.5,
            },
            SentimentResult {
                label: "neutral".into(),
                score: 0.0,
            },
        ];
        let summary = compute_summary(&results);
        assert_eq!(summary.total, 5);
        assert_eq!(summary.positive, 2);
        assert_eq!(summary.negative, 1);
        assert_eq!(summary.neutral, 2);
        assert!((summary.average_score - 0.5 / 5.0).abs() < 1e-10);
    }

    // ─── build_engine ──────────────────────────────────────

    #[tokio::test]
    async fn build_engine_keyword_returns_keyword_engine() {
        let engine = build_engine("keyword");
        let result = engine.analyze("This is great").await.unwrap();
        assert_eq!(result.label, "positive");
    }

    #[tokio::test]
    async fn build_engine_unknown_returns_stub() {
        let engine = build_engine("some_unknown_type");
        let result = engine.analyze("This is great").await.unwrap();
        assert_eq!(result.label, "neutral"); // stub always neutral
    }

    #[tokio::test]
    async fn build_engine_empty_returns_stub() {
        let engine = build_engine("");
        let result = engine.analyze("anything").await.unwrap();
        assert_eq!(result.label, "neutral");
    }
}
