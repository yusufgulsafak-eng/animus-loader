-- 007_patch_archive_sources.sql
-- AdminService::createPatchVersion() ve panelData() bu iki kolona yazıp okuyor
-- ama 001_schema.sql içinde tanımlı değillerdi: temiz kurulumda fatal hata veriyordu.
-- Idempotent: kolon zaten varsa (elle eklenmiş canlı DB) hiçbir şey yapmaz.

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
  "ALTER TABLE patch_archives ADD COLUMN external_url VARCHAR(1000) NULL AFTER source_type",
  'DO 0'
);
PREPARE stmt FROM @sql; EXECUTE stmt; DEALLOCATE PREPARE stmt;

-- Storage GC'nin "diskte dosyası olan" arşivleri hızlı bulabilmesi için.
SET @sql := IF(
  (SELECT COUNT(*) FROM information_schema.STATISTICS
     WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'patch_archives' AND INDEX_NAME = 'idx_archive_source') = 0,
  'ALTER TABLE patch_archives ADD INDEX idx_archive_source (source_type)',
  'DO 0'
);
PREPARE stmt FROM @sql; EXECUTE stmt; DEALLOCATE PREPARE stmt;

-- Harici kaynaklı arşivlerde storage_name gerçek bir dosyaya işaret etmiyor.
UPDATE patch_archives SET source_type = 'external'
WHERE external_url IS NOT NULL AND external_url <> '' AND source_type = 'server';
