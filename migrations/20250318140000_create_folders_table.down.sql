-- Remove folder_id from downloads
DROP INDEX IF EXISTS idx_downloads_folder_id;
ALTER TABLE downloads DROP COLUMN IF EXISTS folder_id;

-- Drop folders table
DROP INDEX IF EXISTS idx_folders_path;
DROP INDEX IF EXISTS idx_folders_parent_id;
DROP INDEX IF EXISTS idx_folders_user_id;
DROP TABLE IF EXISTS folders;
