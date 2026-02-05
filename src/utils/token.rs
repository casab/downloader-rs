//! Token generation and validation utilities.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};

/// Length of the random token in bytes (32 bytes = 256 bits).
const TOKEN_BYTES: usize = 32;

/// Generate a cryptographically secure random token.
///
/// Returns a URL-safe base64 encoded string.
#[must_use]
pub fn generate_token() -> String {
    use rand::RngCore;

    let mut bytes = [0u8; TOKEN_BYTES];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Hash a token for storage.
///
/// Uses SHA-256 since tokens are already high-entropy random values
/// and don't need the slow hashing required for passwords.
#[must_use]
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

/// Verify a token against its hash.
#[must_use]
pub fn verify_token(token: &str, hash: &str) -> bool {
    hash_token(token) == hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_token_length() {
        let token = generate_token();
        // 32 bytes encoded in base64 = 43 characters (no padding)
        assert_eq!(token.len(), 43);
    }

    #[test]
    fn test_generate_token_uniqueness() {
        let token1 = generate_token();
        let token2 = generate_token();
        assert_ne!(token1, token2);
    }

    #[test]
    fn test_hash_token_consistency() {
        let token = "test_token_12345";
        let hash1 = hash_token(token);
        let hash2 = hash_token(token);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_verify_token() {
        let token = generate_token();
        let hash = hash_token(&token);
        assert!(verify_token(&token, &hash));
        assert!(!verify_token("wrong_token", &hash));
    }

    #[test]
    fn test_hash_token_format() {
        let token = "test";
        let hash = hash_token(token);
        // SHA-256 produces 64 hex characters
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
