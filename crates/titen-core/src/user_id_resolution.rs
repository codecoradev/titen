//! Self-healing user_id resolution (#265).
//!
//! Some accounts carry a NULL/malformed `user_id` in the DB (an OAuth
//! exchange path historically failed to persist it — the ajianaz account
//! shipped that way for weeks). Every Graph URL is built as
//! `/v1.0/{user_id}/...`, so a NULL id makes Titen use the raw access token
//! as the path segment and Meta rejects with "Unsupported get request". The
//! token itself is fine — which `GET /me` proves — so the account can heal
//! itself: resolve the real id once, persist it, and never pay the cost
//! again.
//!
//! The actual `ensure_resolved_user_id` implementation lives in
//! `threads_client` (same module as the private `store` field it persists
//! through). This module holds the usability predicate and its tests.

/// A user_id is usable when it is a non-empty numeric string. Meta ids are
/// numeric; the observed failure mode stores the raw TOKEN in the id slot,
/// which is base64-ish and always contains non-digits.
pub fn is_user_id_usable(user_id: &str) -> bool {
    !user_id.is_empty() && user_id.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::is_user_id_usable;

    #[test]
    fn numeric_ids_are_usable() {
        assert!(is_user_id_usable("34327938090154478"));
        assert!(is_user_id_usable("1"));
    }

    #[test]
    fn empty_is_not_usable() {
        assert!(!is_user_id_usable(""));
    }

    #[test]
    fn raw_token_as_id_is_not_usable() {
        // The observed production failure mode: the token stored in the
        // user_id slot (ajianaz account, 2026-09-10). Synthetic value with
        // the same shape — a real token must never be embedded in tests.
        assert!(!is_user_id_usable(
            "THXXSYNT0HET1CEXAMPL3TOKEN0PLACEH0LD3RVALUE0ONLY0FOR0TESTS0XX"
        ));
    }

    #[test]
    fn alphanumeric_ids_are_not_usable() {
        assert!(!is_user_id_usable("17ab34"));
    }
}
