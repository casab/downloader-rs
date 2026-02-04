-- Drop download_tags junction table
DROP INDEX IF EXISTS idx_download_tags_tag_id;
DROP TABLE IF EXISTS download_tags;

-- Drop tags table
DROP INDEX IF EXISTS idx_tags_user_id;
DROP TABLE IF EXISTS tags;
