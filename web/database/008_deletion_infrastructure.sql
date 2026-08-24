-- 008_deletion_infrastructure.sql
-- Kalıcı silme altyapısı: karantina kuyruğu + silme performansı için indeksler.

-- Dosya sistemi işlemleri transaction'a giremez. DB commit edildikten sonra
-- taşınamayan/silinemeyen dosyalar buraya düşer ve maintenance script'i tekrar dener.
CREATE TABLE IF NOT EXISTS storage_gc_queue (
  id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  area ENUM('patch','loader','image','branding') NOT NULL,
  storage_name VARCHAR(500) NOT NULL,
  absolute_path VARCHAR(1000) NOT NULL,
  reason VARCHAR(190) NOT NULL DEFAULT 'delete',
  attempts INT UNSIGNED NOT NULL DEFAULT 0,
  last_error VARCHAR(500) NULL,
  resolved_at DATETIME NULL,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  UNIQUE KEY uq_gc_path (absolute_path(255)),
  INDEX idx_gc_pending (resolved_at, attempts)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- Silme öncesi etki raporu (kaç sürüm, kaç indirme) bu kolonlar üzerinden sayılıyor.
SET @sql := IF(
  (SELECT COUNT(*) FROM information_schema.STATISTICS
     WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'patch_versions' AND INDEX_NAME = 'idx_pv_status') = 0,
  'ALTER TABLE patch_versions ADD INDEX idx_pv_status (status, channel)',
  'DO 0'
);
PREPARE stmt FROM @sql; EXECUTE stmt; DEALLOCATE PREPARE stmt;

SET @sql := IF(
  (SELECT COUNT(*) FROM information_schema.STATISTICS
     WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'download_logs' AND INDEX_NAME = 'idx_dl_archive') = 0,
  'ALTER TABLE download_logs ADD INDEX idx_dl_archive (patch_archive_id)',
  'DO 0'
);
PREPARE stmt FROM @sql; EXECUTE stmt; DEALLOCATE PREPARE stmt;

SET @sql := IF(
  (SELECT COUNT(*) FROM information_schema.STATISTICS
     WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'loader_versions' AND INDEX_NAME = 'idx_loader_channel') = 0,
  'ALTER TABLE loader_versions ADD INDEX idx_loader_channel (channel, published_at)',
  'DO 0'
);
PREPARE stmt FROM @sql; EXECUTE stmt; DEALLOCATE PREPARE stmt;
