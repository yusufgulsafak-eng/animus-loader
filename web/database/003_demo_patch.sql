INSERT INTO patches(game_id,name)
SELECT id,'Demo Türkçe Yama' FROM games WHERE slug='neon-horizon'
AND NOT EXISTS(SELECT 1 FROM patches p JOIN games g2 ON g2.id=p.game_id WHERE g2.slug='neon-horizon');

INSERT INTO patch_versions(patch_id,version,game_version,changelog,minimum_loader_version,status,channel,mandatory_update,access_type,schema_version,published_at)
SELECT p.id,'1.0.0','demo-1.0','İlk telifsiz demo patch paketi.','0.1.0','PUBLISHED','stable',0,'free',1,NOW()
FROM patches p JOIN games g ON g.id=p.game_id WHERE g.slug='neon-horizon'
AND NOT EXISTS(SELECT 1 FROM patch_versions pv WHERE pv.patch_id=p.id AND pv.version='1.0.0' AND pv.channel='stable');

INSERT INTO patch_archives(patch_version_id,storage_name,original_name,mime_type,sha256,size_bytes,file_tree)
SELECT pv.id,'demo-neon-1.0.0.zip','demo-neon-1.0.0.zip','application/zip','4F6DF879F6EE747E1515F9D4F2771923BBF9C5D70DEB039869819E64574539A9',284,
JSON_ARRAY(JSON_OBJECT('path','files/translation.dat','size',152,'directory',false))
FROM patch_versions pv JOIN patches p ON p.id=pv.patch_id JOIN games g ON g.id=p.game_id
WHERE g.slug='neon-horizon' AND pv.version='1.0.0' AND pv.channel='stable'
AND NOT EXISTS(SELECT 1 FROM patch_archives pa WHERE pa.patch_version_id=pv.id);

INSERT INTO patch_install_actions(patch_version_id,action_uuid,action_type,source_path,destination_path,backup_enabled,sort_order,options_json)
SELECT pv.id,'550e8400-e29b-41d4-a716-446655440000','REPLACE_FILE','files/translation.dat','Localization/translation.dat',1,10,JSON_OBJECT()
FROM patch_versions pv JOIN patches p ON p.id=pv.patch_id JOIN games g ON g.id=p.game_id
WHERE g.slug='neon-horizon' AND pv.version='1.0.0' AND pv.channel='stable'
AND NOT EXISTS(SELECT 1 FROM patch_install_actions a WHERE a.patch_version_id=pv.id);

INSERT INTO patch_release_channels(patch_id,channel,active_patch_version_id)
SELECT p.id,'stable',pv.id FROM patches p JOIN games g ON g.id=p.game_id JOIN patch_versions pv ON pv.patch_id=p.id AND pv.version='1.0.0' AND pv.channel='stable'
WHERE g.slug='neon-horizon'
ON DUPLICATE KEY UPDATE active_patch_version_id=VALUES(active_patch_version_id);
