//! Tolerant timestamp parsing and canonicalization.
//!
//! Timestamps reach Titen from several producers with different formats:
//! the Threads Graph API (`+0000` offset style), SQLite `datetime('now')`
//! defaults (`YYYY-MM-DD HH:MM:SS`, no offset), and Titen itself
//! (`chrono::Utc::now().to_rfc3339()`). Lexicographic comparisons in SQL
//! filters and strict RFC3339 parsers both break on the mix, so every
//! write path canonicalizes through [`to_rfc3339_utc`] and read paths that
//! cannot trust stored data use [`parse_utc`].

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};

/// Canonical storage format: compact RFC3339 UTC (`2026-08-27T12:34:56Z`).
pub const CANONICAL_FMT: &str = "%Y-%m-%dT%H:%M:%SZ";

/// Parse a timestamp string in any of the formats Titen produces or
/// ingests, returning a UTC [`DateTime`].
///
/// Accepted inputs:
/// - RFC3339 / ISO-8601 with any valid offset (`...+00:00`, `...+0000`, `...Z`)
/// - `YYYY-MM-DDTHH:MM:SS` (no offset — assumed UTC)
/// - `YYYY-MM-DD HH:MM:SS` (SQLite `datetime('now')` — assumed UTC)
///
/// Fractional seconds are accepted where chrono permits them.
pub fn parse_utc(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    // Threads Graph API style: `2024-07-02T10:30:00+0000` (offset without colon).
    if let Ok(dt) = DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%z") {
        return Some(dt.with_timezone(&Utc));
    }
    // Naive forms — SQLite produces the space variant via datetime('now').
    let naive = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
        .ok()?;
    Utc.from_utc_datetime(&naive).into()
}

/// Canonicalize any accepted timestamp to the compact RFC3339 UTC form.
/// Returns `None` when the input cannot be parsed.
pub fn to_rfc3339_utc(s: &str) -> Option<String> {
    parse_utc(s).map(|dt| dt.format(CANONICAL_FMT).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rfc3339_with_colon_offset() {
        let dt = parse_utc("2026-08-27T12:34:56+00:00").unwrap();
        assert_eq!(dt.format(CANONICAL_FMT).to_string(), "2026-08-27T12:34:56Z");
    }

    #[test]
    fn parses_threads_api_zero_offset() {
        let dt = parse_utc("2024-07-02T10:30:00+0000").unwrap();
        assert_eq!(dt.format(CANONICAL_FMT).to_string(), "2024-07-02T10:30:00Z");
    }

    #[test]
    fn parses_non_utc_offset() {
        let dt = parse_utc("2024-07-02T17:30:00+0700").unwrap();
        assert_eq!(dt.format(CANONICAL_FMT).to_string(), "2024-07-02T10:30:00Z");
    }

    #[test]
    fn parses_z_suffix() {
        let dt = parse_utc("2026-01-01T00:00:00Z").unwrap();
        assert_eq!(dt.format(CANONICAL_FMT).to_string(), "2026-01-01T00:00:00Z");
    }

    #[test]
    fn parses_sqlite_space_format_as_utc() {
        let dt = parse_utc("2026-08-27 12:34:56").unwrap();
        assert_eq!(dt.format(CANONICAL_FMT).to_string(), "2026-08-27T12:34:56Z");
    }

    #[test]
    fn parses_naive_t_format_as_utc() {
        let dt = parse_utc("2026-08-27T12:34:56").unwrap();
        assert_eq!(dt.format(CANONICAL_FMT).to_string(), "2026-08-27T12:34:56Z");
    }

    #[test]
    fn rejects_garbage_and_empty() {
        assert!(parse_utc("").is_none());
        assert!(parse_utc("not a date").is_none());
        assert!(parse_utc("2026-13-45T99:99:99Z").is_none());
    }

    #[test]
    fn canonicalize_roundtrip() {
        assert_eq!(
            to_rfc3339_utc("2024-07-02T10:30:00+0000").as_deref(),
            Some("2024-07-02T10:30:00Z")
        );
        assert!(to_rfc3339_utc("garbage").is_none());
    }
}
