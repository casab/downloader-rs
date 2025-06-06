-- Add up migration script here
CREATE TABLE IF NOT EXISTS downloads (
    id UUID PRIMARY KEY,
    url TEXT NOT NULL,
    status TEXT NOT NULL,
    file_path TEXT,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP WITH TIME ZONE
);