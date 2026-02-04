use chrono::{DateTime, Utc};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};

use sqlx::types::Uuid;

/// Database row representation of a user (for sqlx::FromRow).
#[derive(Debug, sqlx::FromRow)]
pub struct UserRow {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub timezone: Option<String>,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<UserRow> for User {
    fn from(row: UserRow) -> Self {
        Self {
            id: row.id,
            email: row.email,
            password_hash: SecretString::new(row.password_hash.into()),
            display_name: row.display_name,
            avatar_url: row.avatar_url,
            bio: row.bio,
            timezone: row.timezone,
            email_verified_at: row.email_verified_at,
            deleted_at: row.deleted_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Full user record from the database.
#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: SecretString,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub timezone: Option<String>,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// Check if the user's email is verified.
    #[must_use]
    pub fn is_email_verified(&self) -> bool {
        self.email_verified_at.is_some()
    }

    /// Check if the user account is deleted (soft delete).
    #[must_use]
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
}

/// User profile response (safe to return to client).
#[derive(Debug, Serialize)]
pub struct UserProfile {
    pub id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub timezone: String,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<User> for UserProfile {
    fn from(user: User) -> Self {
        let email_verified = user.is_email_verified();
        Self {
            id: user.id,
            email: user.email,
            display_name: user.display_name,
            avatar_url: user.avatar_url,
            bio: user.bio,
            timezone: user.timezone.unwrap_or_else(|| "UTC".to_string()),
            email_verified,
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}

/// Request to update user profile.
#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub bio: Option<String>,
    #[serde(default)]
    pub timezone: Option<String>,
}

impl UpdateProfileRequest {
    /// Check if any fields are set for update.
    #[must_use]
    pub fn has_updates(&self) -> bool {
        self.display_name.is_some()
            || self.avatar_url.is_some()
            || self.bio.is_some()
            || self.timezone.is_some()
    }
}

/// Request to change password.
#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

/// Request to initiate password reset.
#[derive(Debug, Deserialize)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

/// Request to complete password reset.
#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub new_password: String,
}

/// Request to verify email.
#[derive(Debug, Deserialize)]
pub struct VerifyEmailRequest {
    pub token: String,
}
