-- Revert user profile fields
DROP INDEX IF EXISTS idx_users_email_verified;
DROP INDEX IF EXISTS idx_users_deleted_at;

ALTER TABLE users
DROP COLUMN IF EXISTS display_name,
DROP COLUMN IF EXISTS avatar_url,
DROP COLUMN IF EXISTS bio,
DROP COLUMN IF EXISTS timezone,
DROP COLUMN IF EXISTS email_verified_at,
DROP COLUMN IF EXISTS deleted_at;
