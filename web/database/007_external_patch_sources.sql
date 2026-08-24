ALTER TABLE patch_archives
  ADD COLUMN source_type ENUM('server','external') NOT NULL DEFAULT 'server' AFTER patch_version_id,
  ADD COLUMN external_url TEXT NULL AFTER source_type;

UPDATE patch_archives SET source_type='server' WHERE source_type IS NULL OR source_type='';
