-- Add user profile fields for enhanced user management
ALTER TABLE users
ADD COLUMN display_name TEXT,
ADD COLUMN avatar_url TEXT,
ADD COLUMN bio TEXT,
ADD COLUMN timezone TEXT DEFAULT 'UTC',
ADD COLUMN email_verified_at TIMESTAMP WITH TIME ZONE,
ADD COLUMN deleted_at TIMESTAMP WITH TIME ZONE;

-- Index for soft delete queries (only query non-deleted users)
CREATE INDEX idx_users_deleted_at ON users(deleted_at) WHERE deleted_at IS NULL;

-- Index for email verification status
CREATE INDEX idx_users_email_verified ON users(email_verified_at) WHERE email_verified_at IS NOT NULL;

COMMENT ON COLUMN users.display_name IS 'User-chosen display name';
COMMENT ON COLUMN users.avatar_url IS 'URL to user avatar image';
COMMENT ON COLUMN users.bio IS 'User biography/description';
COMMENT ON COLUMN users.timezone IS 'User preferred timezone (IANA format)';
COMMENT ON COLUMN users.email_verified_at IS 'Timestamp when email was verified';
COMMENT ON COLUMN users.deleted_at IS 'Soft delete timestamp';
