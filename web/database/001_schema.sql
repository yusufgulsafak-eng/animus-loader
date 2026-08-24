SET NAMES utf8mb4;
SET time_zone = '+00:00';

CREATE TABLE IF NOT EXISTS users (
  id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  email VARCHAR(190) NOT NULL UNIQUE,
  password_hash VARCHAR(255) NOT NULL,
  display_name VARCHAR(100) NOT NULL,
  role ENUM('user','tester','admin','super_admin') NOT NULL DEFAULT 'user',
  release_channel ENUM('stable','beta','internal') NOT NULL DEFAULT 'stable',
  status ENUM('active','suspended','pending') NOT NULL DEFAULT 'active',
  email_verified_at DATETIME NULL,
  last_login_at DATETIME NULL,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  INDEX idx_users_role_status (role, status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS subscriptions (
  id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  user_id BIGINT UNSIGNED NOT NULL,
  plan_name VARCHAR(100) NOT NULL,
  status ENUM('active','expired','cancelled','trial') NOT NULL,
  starts_at DATETIME NOT NULL,
  ends_at DATETIME NULL,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_sub_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
  INDEX idx_sub_active (user_id, status, ends_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS categories (
  id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  name VARCHAR(100) NOT NULL,
  slug VARCHAR(120) NOT NULL UNIQUE,
  is_active TINYINT(1) NOT NULL DEFAULT 1,
  sort_order INT NOT NULL DEFAULT 0,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS games (
  id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  name VARCHAR(190) NOT NULL,
  slug VARCHAR(190) NOT NULL UNIQUE,
  short_description VARCHAR(500) NULL,
  description TEXT NULL,
  cover_path VARCHAR(500) NULL,
  banner_path VARCHAR(500) NULL,
  steam_app_id VARCHAR(32) NULL,
  epic_catalog_id VARCHAR(190) NULL,
  executable VARCHAR(255) NOT NULL,
  process_name VARCHAR(255) NULL,
  access_type ENUM('free','premium') NOT NULL DEFAULT 'free',
  translation_percent TINYINT UNSIGNED NOT NULL DEFAULT 0,
  minimum_loader_version VARCHAR(32) NOT NULL DEFAULT '0.1.0',
  supported_stores JSON NULL,
  is_active TINYINT(1) NOT NULL DEFAULT 0,
  created_by BIGINT UNSIGNED NULL,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  CONSTRAINT fk_games_creator FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE SET NULL,
  INDEX idx_games_catalog (is_active, access_type, name),
  INDEX idx_games_steam (steam_app_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS game_categories (
  game_id BIGINT UNSIGNED NOT NULL,
  category_id BIGINT UNSIGNED NOT NULL,
  PRIMARY KEY (game_id, category_id),
  CONSTRAINT fk_gc_game FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE,
  CONSTRAINT fk_gc_category FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS game_detection_rules (
  id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  game_id BIGINT UNSIGNED NOT NULL,
  provider ENUM('steam','epic','manual') NOT NULL,
  rule_type ENUM('app_id','expected_path','required_file','optional_file','file_hash','version_file') NOT NULL,
  rule_value VARCHAR(1000) NOT NULL,
  expected_hash CHAR(64) NULL,
  sort_order INT NOT NULL DEFAULT 0,
  is_required TINYINT(1) NOT NULL DEFAULT 1,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_detection_game FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE,
  INDEX idx_detection_game (game_id, provider, sort_order)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS patches (
  id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  game_id BIGINT UNSIGNED NOT NULL,
  name VARCHAR(190) NOT NULL DEFAULT 'Türkçe Yama',
  description TEXT NULL,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  CONSTRAINT fk_patch_game FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE,
  INDEX idx_patch_game (game_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS patch_versions (
  id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  patch_id BIGINT UNSIGNED NOT NULL,
  version VARCHAR(32) NOT NULL,
  game_version VARCHAR(100) NULL,
  changelog TEXT NULL,
  minimum_loader_version VARCHAR(32) NOT NULL DEFAULT '0.1.0',
  status ENUM('DRAFT','TESTING','PUBLISHED','DISABLED','ARCHIVED') NOT NULL DEFAULT 'DRAFT',
  channel ENUM('stable','beta','internal') NOT NULL DEFAULT 'internal',
  mandatory_update TINYINT(1) NOT NULL DEFAULT 0,
  access_type ENUM('free','premium') NOT NULL DEFAULT 'free',
  schema_version SMALLINT UNSIGNED NOT NULL DEFAULT 1,
  manifest_snapshot JSON NULL,
  published_at DATETIME NULL,
  created_by BIGINT UNSIGNED NULL,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  CONSTRAINT fk_pv_patch FOREIGN KEY (patch_id) REFERENCES patches(id) ON DELETE CASCADE,
  CONSTRAINT fk_pv_creator FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE SET NULL,
  UNIQUE KEY uq_patch_version_channel (patch_id, version, channel),
  INDEX idx_pv_release (patch_id, channel, status, published_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS patch_archives (
  id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  patch_version_id BIGINT UNSIGNED NOT NULL UNIQUE,
  storage_name VARCHAR(255) NOT NULL UNIQUE,
  original_name VARCHAR(255) NOT NULL,
  mime_type VARCHAR(100) NOT NULL,
  sha256 CHAR(64) NOT NULL,
  size_bytes BIGINT UNSIGNED NOT NULL,
  file_tree JSON NULL,
  uploaded_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_archive_version FOREIGN KEY (patch_version_id) REFERENCES patch_versions(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS patch_install_actions (
  id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  patch_version_id BIGINT UNSIGNED NOT NULL,
  action_uuid CHAR(36) NOT NULL,
  action_type ENUM('COPY_FILE','COPY_DIRECTORY','REPLACE_FILE','DELETE_FILE','DELETE_DIRECTORY','CREATE_DIRECTORY','MOVE_FILE','RENAME_FILE') NOT NULL,
  source_path VARCHAR(1000) NULL,
  destination_path VARCHAR(1000) NOT NULL,
  backup_enabled TINYINT(1) NOT NULL DEFAULT 1,
  expected_sha256 CHAR(64) NULL,
  sort_order INT NOT NULL,
  options_json JSON NULL,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_action_version FOREIGN KEY (patch_version_id) REFERENCES patch_versions(id) ON DELETE CASCADE,
  UNIQUE KEY uq_action_uuid (patch_version_id, action_uuid),
  INDEX idx_action_order (patch_version_id, sort_order)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS patch_release_channels (
  id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  patch_id BIGINT UNSIGNED NOT NULL,
  channel ENUM('stable','beta','internal') NOT NULL,
  active_patch_version_id BIGINT UNSIGNED NOT NULL,
  updated_by BIGINT UNSIGNED NULL,
  updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  CONSTRAINT fk_prc_patch FOREIGN KEY (patch_id) REFERENCES patches(id) ON DELETE CASCADE,
  CONSTRAINT fk_prc_version FOREIGN KEY (active_patch_version_id) REFERENCES patch_versions(id) ON DELETE RESTRICT,
  CONSTRAINT fk_prc_user FOREIGN KEY (updated_by) REFERENCES users(id) ON DELETE SET NULL,
  UNIQUE KEY uq_patch_channel (patch_id, channel)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS patch_templates (
  id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  name VARCHAR(190) NOT NULL,
  engine_type VARCHAR(100) NOT NULL,
  actions_json JSON NOT NULL,
  is_active TINYINT(1) NOT NULL DEFAULT 1,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS announcements (
  id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  title VARCHAR(190) NOT NULL,
  body TEXT NOT NULL,
  audience ENUM('all','free','premium','tester','admin') NOT NULL DEFAULT 'all',
  is_active TINYINT(1) NOT NULL DEFAULT 1,
  starts_at DATETIME NULL,
  ends_at DATETIME NULL,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS banners (
  id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  title VARCHAR(190) NOT NULL,
  image_path VARCHAR(500) NOT NULL,
  target_url VARCHAR(1000) NULL,
  sort_order INT NOT NULL DEFAULT 0,
  is_active TINYINT(1) NOT NULL DEFAULT 1,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS loader_config (
  id TINYINT UNSIGNED PRIMARY KEY DEFAULT 1,
  app_name VARCHAR(190) NOT NULL,
  logo_url VARCHAR(500) NULL,
  banner_url VARCHAR(500) NULL,
  login_background_url VARCHAR(500) NULL,
  accent_color CHAR(7) NOT NULL DEFAULT '#B7F34A',
  library_title VARCHAR(100) NOT NULL DEFAULT 'Kütüphane',
  discord_url VARCHAR(1000) NULL,
  youtube_url VARCHAR(1000) NULL,
  instagram_url VARCHAR(1000) NULL,
  x_url VARCHAR(1000) NULL,
  support_url VARCHAR(1000) NULL,
  updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS loader_versions (
  id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  version VARCHAR(32) NOT NULL,
  channel ENUM('stable','beta','internal') NOT NULL DEFAULT 'stable',
  storage_name VARCHAR(255) NOT NULL,
  sha256 CHAR(64) NOT NULL,
  size_bytes BIGINT UNSIGNED NOT NULL,
  mandatory TINYINT(1) NOT NULL DEFAULT 0,
  release_notes TEXT NULL,
  published_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE KEY uq_loader_version (version, channel)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS api_tokens (
  id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  user_id BIGINT UNSIGNED NOT NULL,
  token_hash CHAR(64) NOT NULL UNIQUE,
  name VARCHAR(100) NOT NULL DEFAULT 'loader',
  expires_at DATETIME NOT NULL,
  last_used_at DATETIME NULL,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_api_token_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
  INDEX idx_token_expiry (expires_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS download_tokens (
  id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  user_id BIGINT UNSIGNED NOT NULL,
  patch_archive_id BIGINT UNSIGNED NOT NULL,
  token_hash CHAR(64) NOT NULL UNIQUE,
  expires_at DATETIME NOT NULL,
  used_at DATETIME NULL,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_dt_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
  CONSTRAINT fk_dt_archive FOREIGN KEY (patch_archive_id) REFERENCES patch_archives(id) ON DELETE CASCADE,
  INDEX idx_download_token_expiry (expires_at, used_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS download_logs (
  id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  user_id BIGINT UNSIGNED NULL,
  patch_archive_id BIGINT UNSIGNED NULL,
  ip_hash CHAR(64) NOT NULL,
  user_agent VARCHAR(500) NULL,
  bytes_sent BIGINT UNSIGNED NOT NULL DEFAULT 0,
  status ENUM('started','completed','failed') NOT NULL,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_dl_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL,
  CONSTRAINT fk_dl_archive FOREIGN KEY (patch_archive_id) REFERENCES patch_archives(id) ON DELETE SET NULL,
  INDEX idx_download_date (created_at, status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS audit_logs (
  id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  actor_user_id BIGINT UNSIGNED NULL,
  action VARCHAR(100) NOT NULL,
  entity_type VARCHAR(100) NOT NULL,
  entity_id VARCHAR(100) NULL,
  before_json JSON NULL,
  after_json JSON NULL,
  ip_hash CHAR(64) NOT NULL,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_audit_user FOREIGN KEY (actor_user_id) REFERENCES users(id) ON DELETE SET NULL,
  INDEX idx_audit_entity (entity_type, entity_id, created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS rate_limits (
  bucket_key CHAR(64) PRIMARY KEY,
  hits INT UNSIGNED NOT NULL,
  expires_at DATETIME NOT NULL,
  INDEX idx_rate_expiry (expires_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

