-- Remove job_message_id from downloads
DROP INDEX IF EXISTS idx_downloads_job_message_id;
ALTER TABLE downloads DROP COLUMN IF EXISTS job_message_id;

-- Drop job_history table and indexes
DROP INDEX IF EXISTS idx_job_history_status_type;
DROP INDEX IF EXISTS idx_job_history_created_at;
DROP INDEX IF EXISTS idx_job_history_job_type;
DROP INDEX IF EXISTS idx_job_history_status;
DROP INDEX IF EXISTS idx_job_history_user_id;
DROP TABLE IF EXISTS job_history;
