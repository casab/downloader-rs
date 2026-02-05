-- Enhance downloads table with progress tracking and control fields

-- Add new status values by recreating the check or using text type
-- First, add new columns for progress tracking
ALTER TABLE downloads
ADD COLUMN bytes_downloaded BIGINT DEFAULT 0,
ADD COLUMN total_bytes BIGINT,
ADD COLUMN content_type TEXT,
ADD COLUMN filename TEXT,
ADD COLUMN error_message TEXT,
ADD COLUMN retry_count INTEGER DEFAULT 0,
ADD COLUMN max_retries INTEGER DEFAULT 3,
ADD COLUMN priority INTEGER DEFAULT 0,
ADD COLUMN started_at TIMESTAMP WITH TIME ZONE,
ADD COLUMN metadata JSONB DEFAULT '{}';

-- Create index for efficient queue processing (status + priority)
CREATE INDEX idx_downloads_status_priority ON downloads(status, priority DESC);

-- Create index for user's active downloads
CREATE INDEX idx_downloads_user_active ON downloads(user_id, status)
WHERE status IN ('PENDING', 'IN_PROGRESS', 'PAUSED');

COMMENT ON COLUMN downloads.bytes_downloaded IS 'Number of bytes downloaded so far';
COMMENT ON COLUMN downloads.total_bytes IS 'Total size in bytes (null if unknown)';
COMMENT ON COLUMN downloads.content_type IS 'MIME type of the downloaded file';
COMMENT ON COLUMN downloads.filename IS 'Original filename from Content-Disposition or URL';
COMMENT ON COLUMN downloads.error_message IS 'Last error message if failed';
COMMENT ON COLUMN downloads.retry_count IS 'Number of retry attempts made';
COMMENT ON COLUMN downloads.max_retries IS 'Maximum retry attempts allowed';
COMMENT ON COLUMN downloads.priority IS 'Download priority (higher = more priority)';
COMMENT ON COLUMN downloads.started_at IS 'When the download actually started';
COMMENT ON COLUMN downloads.metadata IS 'Additional metadata as JSON';
