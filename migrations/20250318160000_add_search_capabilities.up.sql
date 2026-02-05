-- Add full-text search vector to downloads
ALTER TABLE downloads
ADD COLUMN search_vector tsvector
GENERATED ALWAYS AS (
    setweight(to_tsvector('english', coalesce(filename, '')), 'A') ||
    setweight(to_tsvector('english', coalesce(url, '')), 'B')
) STORED;

CREATE INDEX idx_downloads_search ON downloads USING GIN(search_vector);

-- Add search vector to folders
ALTER TABLE folders
ADD COLUMN search_vector tsvector
GENERATED ALWAYS AS (to_tsvector('english', name)) STORED;

CREATE INDEX idx_folders_search ON folders USING GIN(search_vector);
