-- SAFE STAGING ONLY. DO NOT RUN UNTIL THE NEW ZIP IS HOSTED.
-- Existing published stable id=106 is NOT changed. Creates DRAFT 1.0.1.
-- The 012 migration is already installed on yusulvie_animus_loader.
-- First: upload Battlefield_V_Belgeler_Duzeltme_1.0.1.zip to a NEW HTTPS URL.
-- Change the URL below; do NOT use the old MediaFire URL.
SET @bfv_new_url = 'PASTE_NEW_HTTPS_DOWNLOAD_URL_HERE';
SET NAMES utf8mb4;
DROP PROCEDURE IF EXISTS animus_bfv_stage_documents_fix;
DELIMITER $$
CREATE PROCEDURE animus_bfv_stage_documents_fix()
BEGIN
 DECLARE v_id BIGINT UNSIGNED DEFAULT NULL;
 DECLARE v_p BIGINT UNSIGNED DEFAULT NULL;
 DECLARE v_active BIGINT UNSIGNED DEFAULT NULL;
 DECLARE v_old_hash CHAR(64) DEFAULT NULL;
 DECLARE v_enum LONGTEXT DEFAULT NULL;
 DECLARE EXIT HANDLER FOR SQLEXCEPTION BEGIN ROLLBACK; RESIGNAL; END;
 IF @bfv_new_url IS NULL OR @bfv_new_url LIKE 'PASTE_%'
    OR @bfv_new_url NOT LIKE 'https://%'
    OR @bfv_new_url = 'https://www.mediafire.com/file/r4ozdjjkwjuodky/Battlefield%25E2%2584%25A2_5.zip/file' THEN
   SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT='STOP: Upload new ZIP and set new HTTPS download URL first';
 END IF;
 START TRANSACTION;
 SELECT pv.patch_id INTO v_p FROM patch_versions pv WHERE pv.id=106 FOR UPDATE;
 IF v_p IS NULL OR v_p<>100 THEN SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT='Expected patch 100 version 106 not found'; END IF;
 SELECT prc.active_patch_version_id INTO v_active FROM patch_release_channels prc
  WHERE prc.patch_id=v_p AND prc.channel='stable' FOR UPDATE;
 IF v_active IS NULL OR v_active<>106 THEN SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT='Stable release changed; review before staging'; END IF;
 SELECT LOWER(pa.sha256) INTO v_old_hash FROM patch_archives pa WHERE pa.patch_version_id=106;
 IF v_old_hash <> '7e33a18bacd618079383dd173b7e64b7ccaba0588a844098c089f8be5d753f7c'
   OR v_old_hash IS NULL THEN SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT='Stable archive SHA changed'; END IF;
 SELECT COLUMN_TYPE INTO v_enum FROM information_schema.COLUMNS
  WHERE TABLE_SCHEMA=DATABASE() AND TABLE_NAME='patch_install_actions' AND COLUMN_NAME='action_type';
 IF v_enum IS NULL OR LOCATE(CONCAT(CHAR(39),'RUN_BAT',CHAR(39)),v_enum)=0 THEN
   SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT='RUN_BAT column migration missing'; END IF;
 IF EXISTS(SELECT 1 FROM patch_versions WHERE patch_id=v_p AND version='1.0.1' AND channel='stable') THEN
   SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT='Version 1.0.1 already exists; no duplicate changes'; END IF;
 INSERT INTO patch_versions(patch_id,version,game_version,changelog,minimum_loader_version,status,channel,mandatory_update,access_type,schema_version)
 SELECT patch_id,'1.0.1',game_version,'Windows Known Folder Documents (Belgeler) repair','0.1.3','DRAFT','stable',mandatory_update,access_type,1
 FROM patch_versions WHERE id=106;
 SET v_id=LAST_INSERT_ID();
 INSERT INTO patch_archives(patch_version_id,source_type,external_url,storage_name,original_name,mime_type,sha256,size_bytes,file_tree)
 VALUES(v_id,'external',@bfv_new_url,CONCAT('external-bfv-docs-',v_id,'.zip'),
 'Battlefield_V_Belgeler_Duzeltme_1.0.1.zip','application/zip',
 'cfb33da16e3893e8aea62c5c4599e79da9a3cfe4876e76181d01a1fea6639359',20568583,
 JSON_ARRAY('Data/','Data/layout.toc','Data/mods.json','Data/win32/','Data/win32/installation/',
 'Data/win32/installation/initialexperience/','Data/win32/installation/initialexperience/cas.cat',
 'Data/win32/installation/initialexperience/cas_13.cas','Data/win32/installation/initialexperience/cas_14.cas',
 'CryptBase.dll','dinput8.dll','en_US-74558638.json','Install.bat'));
 INSERT INTO patch_install_actions(patch_version_id,action_uuid,action_type,source_path,destination_path,backup_enabled,expected_sha256,sort_order,options_json)
 VALUES
 (v_id,UUID(),'COPY_DIRECTORY','Data','Data',1,NULL,10,NULL),
 (v_id,UUID(),'COPY_FILE','CryptBase.dll','CryptBase.dll',1,NULL,20,NULL),
 (v_id,UUID(),'COPY_FILE','dinput8.dll','dinput8.dll',1,NULL,30,NULL),
 (v_id,UUID(),'RUN_BAT','Install.bat','Install.bat',0,'7016526b403eff2374a2874274a29bae97fff14daa10682d230f32716d2ff463',90,NULL);
 COMMIT;
 SELECT v_id AS staged_version_id,'DRAFT only; published stable v106 UNCHANGED' AS sonuc;
END$$
DELIMITER ;
CALL animus_bfv_stage_documents_fix();
DROP PROCEDURE IF EXISTS animus_bfv_stage_documents_fix;
