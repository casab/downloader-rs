-- Remove search from folders
DROP INDEX IF EXISTS idx_folders_search;
ALTER TABLE folders DROP COLUMN IF EXISTS search_vector;

-- Remove search from downloads
DROP INDEX IF EXISTS idx_downloads_search;
ALTER TABLE downloads DROP COLUMN IF EXISTS search_vector;
