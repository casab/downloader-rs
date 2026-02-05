-- Revert enhanced downloads table

DROP INDEX IF EXISTS idx_downloads_user_active;
DROP INDEX IF EXISTS idx_downloads_status_priority;

ALTER TABLE downloads
DROP COLUMN IF EXISTS bytes_downloaded,
DROP COLUMN IF EXISTS total_bytes,
DROP COLUMN IF EXISTS content_type,
DROP COLUMN IF EXISTS filename,
DROP COLUMN IF EXISTS error_message,
DROP COLUMN IF EXISTS retry_count,
DROP COLUMN IF EXISTS max_retries,
DROP COLUMN IF EXISTS priority,
DROP COLUMN IF EXISTS started_at,
DROP COLUMN IF EXISTS metadata;
