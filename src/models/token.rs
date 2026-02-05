//! Token models for password reset, email verification, etc.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;

/// Token type enum matching the database enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "token_type", rename_all = "snake_case")]
pub enum TokenType {
    PasswordReset,
    EmailVerification,
    EmailChange,
}

impl TokenType {
    /// Get the default expiration duration for this token type.
    #[must_use]
    pub fn default_expiration(&self) -> Duration {
        match self {
            Self::PasswordReset => Duration::hours(1),
            Self::EmailVerification | Self::EmailChange => Duration::hours(24),
        }
    }
}

/// Token record from the database.
#[derive(Debug, sqlx::FromRow)]
pub struct Token {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub token_type: TokenType,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

impl Token {
    /// Check if the token has expired.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Check if the token has been used.
    #[must_use]
    pub fn is_used(&self) -> bool {
        self.used_at.is_some()
    }

    /// Check if the token is valid (not expired and not used).
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.is_expired() && !self.is_used()
    }
}

/// Metadata for email change tokens.
#[derive(Debug, Serialize, Deserialize)]
pub struct EmailChangeMetadata {
    pub new_email: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_type_expiration() {
        assert_eq!(
            TokenType::PasswordReset.default_expiration(),
            Duration::hours(1)
        );
        assert_eq!(
            TokenType::EmailVerification.default_expiration(),
            Duration::hours(24)
        );
    }
}
