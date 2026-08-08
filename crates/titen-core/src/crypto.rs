//! AES-256-GCM encryption for sensitive fields at rest.
//!
//! Encrypts `access_token` and `app_secret` before writing to SQLite,
//! decrypts transparently on read. The application layer always sees plaintext.
//!
//! ## Storage Format
//!
//! Encrypted values are stored as: `enc:v1:<base64(nonce || ciphertext + tag)>`
//!
//! - `enc:v1:` prefix enables versioning and plaintext detection (backward compat)
//! - 12-byte nonce generated per encryption via cryptographically secure RNG
//! - Ciphertext includes the 16-byte GCM authentication tag
//!
//! ## Key Management
//!
//! The 32-byte key is read from `TITEN_ENCRYPTION_KEY` (hex-encoded, 64 chars).
//! Generate with: `openssl rand -hex 32`
//!
//! If the env var is absent, [`Cipher::from_env`] returns `Ok(None)` — the store
//! operates in **plaintext mode** (dev only). Production callers should check
//! and reject startup if the key is missing.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use rand::RngCore;

use crate::error::{Result, TitenError};

/// Version prefix for encrypted values. Enables future format migration.
const ENCRYPTION_PREFIX: &str = "enc:v1:";

/// AES-256-GCM cipher holder. `Option<Cipher>` in the store allows
/// graceful degradation to plaintext mode in development.
#[derive(Clone)]
pub struct Cipher {
    key: [u8; 32],
}

impl Cipher {
    /// Create a cipher from a raw 32-byte key.
    pub fn new(key: [u8; 32]) -> Self {
        Self { key }
    }

    /// Load the encryption key from `TITEN_ENCRYPTION_KEY` env var.
    ///
    /// Returns `Ok(None)` if the variable is not set (dev mode — plaintext).
    /// Returns `Err` if set but invalid (wrong length, not hex, etc.).
    pub fn from_env() -> Result<Option<Self>> {
        let raw = match std::env::var("TITEN_ENCRYPTION_KEY") {
            Ok(v) => v,
            Err(_) => return Ok(None),
        };

        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }

        let key_bytes = hex::decode(trimmed).map_err(|e| {
            TitenError::ConfigError(format!("TITEN_ENCRYPTION_KEY is not valid hex: {e}"))
        })?;

        if key_bytes.len() != 32 {
            return Err(TitenError::ConfigError(format!(
                "TITEN_ENCRYPTION_KEY must be 32 bytes (64 hex chars), got {} bytes",
                key_bytes.len()
            )));
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&key_bytes);
        Ok(Some(Self { key }))
    }

    /// Encrypt a plaintext string.
    ///
    /// Returns `enc:v1:<base64(nonce || ciphertext)>`.
    /// Empty strings are returned as-is (no encryption needed).
    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        if plaintext.is_empty() {
            return Ok(String::new());
        }

        // Already encrypted — don't double-encrypt
        if plaintext.starts_with(ENCRYPTION_PREFIX) {
            return Ok(plaintext.to_string());
        }

        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.key));

        // Generate a unique 12-byte nonce per encryption
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| TitenError::ConfigError(format!("Encryption failed: {e}")))?;

        // Prepend nonce to ciphertext for storage
        let mut combined = Vec::with_capacity(nonce_bytes.len() + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);

        Ok(format!("{ENCRYPTION_PREFIX}{}", BASE64.encode(&combined)))
    }

    /// Decrypt an encrypted string back to plaintext.
    ///
    /// If the value doesn't have the `enc:v1:` prefix, it's returned as-is
    /// (plaintext mode — for dev or pre-migration data).
    pub fn decrypt(&self, stored: &str) -> Result<String> {
        if stored.is_empty() || !stored.starts_with(ENCRYPTION_PREFIX) {
            // Plaintext — return as-is (dev mode or not yet migrated)
            return Ok(stored.to_string());
        }

        let encoded = &stored[ENCRYPTION_PREFIX.len()..];
        let combined = BASE64.decode(encoded).map_err(|e| {
            TitenError::ConfigError(format!("Failed to decode encrypted value: {e}"))
        })?;

        if combined.len() < 12 {
            return Err(TitenError::ConfigError(
                "Encrypted value too short (missing nonce)".to_string(),
            ));
        }

        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.key));
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|_| {
            TitenError::ConfigError("Decryption failed — wrong key or tampered data".to_string())
        })?;

        String::from_utf8(plaintext).map_err(|e| {
            TitenError::ConfigError(format!("Decrypted value is not valid UTF-8: {e}"))
        })
    }
}

/// Check if a stored value is encrypted (has the `enc:v1:` prefix).
pub fn is_encrypted(value: &str) -> bool {
    value.starts_with(ENCRYPTION_PREFIX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn test_cipher() -> Cipher {
        // Deterministic test key — 32 bytes of 0xAA
        Cipher::new([0xAA; 32])
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let cipher = test_cipher();
        let plaintext = "EAAGm0x secret_threads_token_12345";
        let encrypted = cipher.encrypt(plaintext).unwrap();

        assert!(encrypted.starts_with("enc:v1:"));
        assert_ne!(encrypted, plaintext);

        let decrypted = cipher.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_each_encryption_is_unique() {
        // Same plaintext produces different ciphertext (random nonce)
        let cipher = test_cipher();
        let plaintext = "same_secret";
        let enc1 = cipher.encrypt(plaintext).unwrap();
        let enc2 = cipher.encrypt(plaintext).unwrap();

        assert_ne!(enc1, enc2, "Nonce should be random per encryption");
        assert_eq!(cipher.decrypt(&enc1).unwrap(), plaintext);
        assert_eq!(cipher.decrypt(&enc2).unwrap(), plaintext);
    }

    #[test]
    fn test_empty_string_passthrough() {
        let cipher = test_cipher();
        let encrypted = cipher.encrypt("").unwrap();
        assert_eq!(encrypted, "");
        let decrypted = cipher.decrypt("").unwrap();
        assert_eq!(decrypted, "");
    }

    #[test]
    fn test_plaintext_passthrough_on_decrypt() {
        // Values without prefix should pass through (backward compat)
        let cipher = test_cipher();
        let result = cipher.decrypt("not_encrypted_token").unwrap();
        assert_eq!(result, "not_encrypted_token");
    }

    #[test]
    fn test_no_double_encryption() {
        let cipher = test_cipher();
        let plaintext = "my_secret";
        let encrypted = cipher.encrypt(plaintext).unwrap();
        let double = cipher.encrypt(&encrypted).unwrap();
        assert_eq!(
            encrypted, double,
            "Should not re-encrypt already encrypted data"
        );
    }

    #[test]
    fn test_tamper_detection() {
        let cipher = test_cipher();
        let encrypted = cipher.encrypt("secret").unwrap();

        // Corrupt the base64 payload
        let mut tampered = encrypted.clone();
        // Replace last char to corrupt the ciphertext
        let last_char = tampered.chars().last().unwrap();
        let replacement = if last_char == 'A' { 'B' } else { 'A' };
        tampered.pop();
        tampered.push(replacement);

        let result = cipher.decrypt(&tampered);
        assert!(
            result.is_err(),
            "Tampered ciphertext should fail decryption"
        );
    }

    #[test]
    fn test_wrong_key_fails() {
        let cipher1 = Cipher::new([0xAA; 32]);
        let cipher2 = Cipher::new([0xBB; 32]);

        let encrypted = cipher1.encrypt("secret").unwrap();
        let result = cipher2.decrypt(&encrypted);
        assert!(result.is_err(), "Decryption with wrong key should fail");
    }

    #[test]
    fn test_is_encrypted_helper() {
        assert!(is_encrypted("enc:v1:abc123"));
        assert!(!is_encrypted("plaintext_token"));
        assert!(!is_encrypted(""));
    }

    #[test]
    #[serial]
    fn test_from_env_missing() {
        unsafe { std::env::remove_var("TITEN_ENCRYPTION_KEY") };
        let result = Cipher::from_env().unwrap();
        assert!(result.is_none(), "Missing env var should return None");
    }

    #[test]
    #[serial]
    fn test_from_env_empty() {
        unsafe { std::env::set_var("TITEN_ENCRYPTION_KEY", "") };
        let result = Cipher::from_env().unwrap();
        assert!(result.is_none(), "Empty env var should return None");
        unsafe { std::env::remove_var("TITEN_ENCRYPTION_KEY") };
    }

    #[test]
    #[serial]
    fn test_from_env_valid() {
        let key = "aa".repeat(32); // 64 hex chars = 32 bytes
        unsafe { std::env::set_var("TITEN_ENCRYPTION_KEY", &key) };
        let cipher = Cipher::from_env().unwrap().expect("Should produce cipher");

        // Verify it works
        let enc = cipher.encrypt("test").unwrap();
        let dec = cipher.decrypt(&enc).unwrap();
        assert_eq!(dec, "test");

        unsafe { std::env::remove_var("TITEN_ENCRYPTION_KEY") };
    }

    #[test]
    #[serial]
    fn test_from_env_wrong_length() {
        unsafe { std::env::set_var("TITEN_ENCRYPTION_KEY", "aabbcc") }; // Too short
        let result = Cipher::from_env();
        assert!(result.is_err(), "Short key should error");
        unsafe { std::env::remove_var("TITEN_ENCRYPTION_KEY") };
    }

    #[test]
    #[serial]
    fn test_from_env_not_hex() {
        unsafe {
            std::env::set_var(
                "TITEN_ENCRYPTION_KEY",
                "not_hex_at_all_just_random_text_!!!___",
            )
        };
        let result = Cipher::from_env();
        assert!(result.is_err(), "Non-hex key should error");
        unsafe { std::env::remove_var("TITEN_ENCRYPTION_KEY") };
    }

    #[test]
    fn test_long_token_roundtrip() {
        let cipher = test_cipher();
        // Simulate a realistic Meta Threads long-lived access token
        let token = "EAAGm0x4ZCphBPnHk9NxTL2wJ5vQR8sF3iKd7M1oZuWvYbAcDeFgHiJkLmNoPqRsTuVwXyZ0123456789abcdefghijklmnopqrstuvwxyz";
        let encrypted = cipher.encrypt(token).unwrap();
        let decrypted = cipher.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, token);
    }
}
