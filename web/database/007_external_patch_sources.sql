-- 007_external_patch_sources.sql
-- Harici (PixelDrain / CDN) patch kaynaklari icin kolonlar.
--
-- ONEMLI: Bu dosya once duz bir ALTER TABLE idi. Kolonlari elle eklenmis
-- canli veritabanlarinda migrate.php "Duplicate column" hatasiyla duruyordu.
-- Artik idempotent: kolon varsa hicbir sey yapmaz, iki kez calistirmak guvenli.

SET @sql := IF(
  (SELECT COUNT(*) FROM information_schema.COLUMNS
     WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'patch_archives' AND COLUMN_NAME = 'source_type') = 0,
  "ALTER TABLE patch_archives ADD COLUMN source_type ENUM('server','external') NOT NULL DEFAULT 'server' AFTER patch_version_id",
  'DO 0'
);
PREPARE stmt FROM @sql; EXECUTE stmt; DEALLOCATE PREPARE stmt;

SET @sql := IF(
  (SELECT COUNT(*) FROM information_schema.COLUMNS
     WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'patch_archives' AND COLUMN_NAME = 'external_url') = 0,
  "ALTER TABLE patch_archives ADD COLUMN external_url TEXT NULL AFTER source_type",
  'DO 0'
);
PREPARE stmt FROM @sql; EXECUTE stmt; DEALLOCATE PREPARE stmt;

-- Storage GC'nin "diskte gercek dosyasi olan" arsivleri hizli bulabilmesi icin.
SET @sql := IF(
  (SELECT COUNT(*) FROM information_schema.STATISTICS
     WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'patch_archives' AND INDEX_NAME = 'idx_archive_source') = 0,
  'ALTER TABLE patch_archives ADD INDEX idx_archive_source (source_type)',
  'DO 0'
);
PREPARE stmt FROM @sql; EXECUTE stmt; DEALLOCATE PREPARE stmt;

UPDATE patch_archives SET source_type = 'server' WHERE source_type IS NULL OR source_type = '';
UPDATE patch_archives SET source_type = 'external'
WHERE external_url IS NOT NULL AND external_url <> '' AND source_type = 'server';
