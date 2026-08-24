ALTER TABLE loader_config
  ADD COLUMN login_background_type ENUM('default','image','video') NOT NULL DEFAULT 'default' AFTER login_background_url,
  ADD COLUMN login_background_image VARCHAR(500) NULL AFTER login_background_type,
  ADD COLUMN login_background_video VARCHAR(500) NULL AFTER login_background_image,
  ADD COLUMN login_background_fallback VARCHAR(500) NULL AFTER login_background_video,
  ADD COLUMN login_background_overlay TINYINT UNSIGNED NOT NULL DEFAULT 60 AFTER login_background_fallback,
  ADD COLUMN library_background_type ENUM('default','image','video') NOT NULL DEFAULT 'default' AFTER login_background_overlay,
  ADD COLUMN library_background_image VARCHAR(500) NULL AFTER library_background_type,
  ADD COLUMN library_background_video VARCHAR(500) NULL AFTER library_background_image,
  ADD COLUMN library_background_fallback VARCHAR(500) NULL AFTER library_background_video,
  ADD COLUMN library_background_overlay TINYINT UNSIGNED NOT NULL DEFAULT 55 AFTER library_background_fallback;

UPDATE loader_config
SET login_background_type='image',
    login_background_image=login_background_url
WHERE login_background_url IS NOT NULL AND login_background_url<>'';

