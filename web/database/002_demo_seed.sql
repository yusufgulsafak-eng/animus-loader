INSERT INTO loader_config (id, app_name, accent_color, library_title, support_url)
VALUES (1, 'Animus Türkçe Yama', '#B7F34A', 'Oyun Kütüphanesi', 'https://example.com/support')
ON DUPLICATE KEY UPDATE id = id;

INSERT INTO categories (name, slug, sort_order) VALUES
('Aksiyon', 'aksiyon', 10), ('Macera', 'macera', 20), ('Rol Yapma', 'rol-yapma', 30)
ON DUPLICATE KEY UPDATE name = VALUES(name);

INSERT INTO games (name, slug, short_description, description, cover_path, banner_path, steam_app_id, executable, process_name, access_type, translation_percent, supported_stores, is_active) VALUES
('Neon Horizon', 'neon-horizon', 'Uzak bir gelecekte geçen özgün demo oyunu.', 'Loader katalog ve manifest akışını göstermek için oluşturulmuş telifsiz placeholder kaydıdır.', '/assets/placeholders/cover-neon.svg', '/assets/placeholders/banner-neon.svg', '900001', 'NeonHorizon.exe', 'NeonHorizon.exe', 'free', 100, JSON_ARRAY('steam','manual'), 1),
('Echoes of Anatolia', 'echoes-of-anatolia', 'Anadolu esintili özgün macera demosu.', 'Gerçek bir oyuna veya markaya bağlı olmayan test kaydıdır.', '/assets/placeholders/cover-echoes.svg', '/assets/placeholders/banner-echoes.svg', '900002', 'Echoes.exe', 'Echoes.exe', 'premium', 82, JSON_ARRAY('steam','manual'), 1),
('Lime Protocol', 'lime-protocol', 'Siber dünyada geçen özgün strateji demosu.', 'Admin yayın ve loader güncelleme akışları için placeholder içeriktir.', '/assets/placeholders/cover-lime.svg', '/assets/placeholders/banner-lime.svg', NULL, 'LimeProtocol.exe', 'LimeProtocol.exe', 'free', 64, JSON_ARRAY('manual'), 1)
ON DUPLICATE KEY UPDATE name = VALUES(name);

INSERT INTO game_categories (game_id, category_id)
SELECT g.id, c.id FROM games g JOIN categories c ON
 (g.slug='neon-horizon' AND c.slug='aksiyon') OR
 (g.slug='echoes-of-anatolia' AND c.slug='macera') OR
 (g.slug='lime-protocol' AND c.slug='rol-yapma')
ON DUPLICATE KEY UPDATE game_id = game_id;

INSERT INTO game_detection_rules (game_id, provider, rule_type, rule_value, sort_order, is_required)
SELECT id, IF(steam_app_id IS NULL, 'manual', 'steam'), 'required_file', executable, 10, 1 FROM games
WHERE slug IN ('neon-horizon','echoes-of-anatolia','lime-protocol')
  AND NOT EXISTS (SELECT 1 FROM game_detection_rules r WHERE r.game_id=games.id AND r.rule_type='required_file');

INSERT INTO patch_templates (name, engine_type, actions_json) VALUES
('Simple File Replacement', 'generic', JSON_ARRAY(JSON_OBJECT('type','REPLACE_FILE','source','files/translation.dat','destination','Localization/translation.dat','backup',true))),
('Unreal Engine Localization', 'unreal', JSON_ARRAY(JSON_OBJECT('type','COPY_DIRECTORY','source','files/Localization','destination','Content/Localization','backup',true))),
('Unity Localization', 'unity', JSON_ARRAY(JSON_OBJECT('type','COPY_DIRECTORY','source','files/StreamingAssets','destination','Game_Data/StreamingAssets','backup',true))),
('Pak File Patch', 'pak', JSON_ARRAY(JSON_OBJECT('type','COPY_FILE','source','files/turkish_P.pak','destination','Content/Paks/~mods/turkish_P.pak','backup',true)))
ON DUPLICATE KEY UPDATE name = VALUES(name);

