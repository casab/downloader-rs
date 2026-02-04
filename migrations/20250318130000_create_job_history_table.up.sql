-- Create job_history table for tracking job execution history
CREATE TABLE job_history (
    id UUID PRIMARY KEY,
    stream_message_id TEXT,
    job_type TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    payload JSONB NOT NULL,
    result JSONB,
    error_message TEXT,
    priority INTEGER NOT NULL DEFAULT 0,
    attempts INTEGER NOT NULL DEFAULT 0,
    worker_id TEXT,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE
);

-- Indexes for common queries
CREATE INDEX idx_job_history_user_id ON job_history(user_id);
CREATE INDEX idx_job_history_status ON job_history(status);
CREATE INDEX idx_job_history_job_type ON job_history(job_type);
CREATE INDEX idx_job_history_created_at ON job_history(created_at DESC);
CREATE INDEX idx_job_history_status_type ON job_history(status, job_type);

-- Add job_message_id column to downloads table to link downloads to jobs
ALTER TABLE downloads
ADD COLUMN job_message_id TEXT;

CREATE INDEX idx_downloads_job_message_id ON downloads(job_message_id);
