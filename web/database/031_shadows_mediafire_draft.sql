-- ORIJINAL Shadows payload supplied by the project owner, 2026-10-17.
-- Apply 007 and 030 first. Deploy the Shadows PHP validator/admin changes.
-- Creates internal DRAFT 1.0.0-shadows.1; publish through the admin validator.
-- No release channel, user account, or global Loader setting is changed.
SET NAMES utf8mb4;
DROP PROCEDURE IF EXISTS animus_stage_shadows_mediafire;
DELIMITER $$
CREATE PROCEDURE animus_stage_shadows_mediafire()
BEGIN
    DECLARE v_game BIGINT UNSIGNED DEFAULT NULL;
    DECLARE v_patch BIGINT UNSIGNED DEFAULT NULL;
    DECLARE v_version BIGINT UNSIGNED DEFAULT NULL;
    DECLARE v_count INT DEFAULT 0;
    DECLARE v_access VARCHAR(16);
    DECLARE v_executable VARCHAR(255);
    DECLARE v_process VARCHAR(255);
    DECLARE v_minimum VARCHAR(32);
    DECLARE v_enum LONGTEXT DEFAULT NULL;
    DECLARE EXIT HANDLER FOR SQLEXCEPTION
    BEGIN
        ROLLBACK;
        RESIGNAL;
    END;

    SELECT COLUMN_TYPE INTO v_enum FROM information_schema.COLUMNS
    WHERE TABLE_SCHEMA=DATABASE() AND TABLE_NAME='patch_install_actions'
      AND COLUMN_NAME='action_type';
    IF v_enum IS NULL OR LOCATE(CONCAT(CHAR(39),'INSTALL_SHADOWS',CHAR(39)),v_enum)=0 THEN
        SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT='Apply 030_shadows_loader_action.sql first';
    END IF;
    SELECT COUNT(*) INTO v_count FROM information_schema.COLUMNS
    WHERE TABLE_SCHEMA=DATABASE() AND TABLE_NAME='patch_archives'
      AND COLUMN_NAME IN ('source_type','external_url');
    IF v_count<>2 THEN
        SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT='Apply 007_external_patch_sources.sql first';
    END IF;

    START TRANSACTION;
    SELECT COUNT(*),MIN(id) INTO v_count,v_game FROM games
    WHERE slug IN ('assassins-creed-shadows','assassins-creed-shadows-orijinal');
    IF v_count>1 THEN
        SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT='Two original Shadows games found; select one before staging';
    END IF;
    IF v_game IS NULL THEN
        INSERT INTO games(name,slug,short_description,executable,process_name,
                          access_type,minimum_loader_version,supported_stores,is_active)
        VALUES('Assassin''s Creed Shadows','assassins-creed-shadows',
               'ORIJINAL surum Turkce yama test paketi','ACShadows.exe','ACShadows.exe',
               'premium','0.1.5',JSON_ARRAY('manual'),0);
        SET v_game=LAST_INSERT_ID();
    END IF;
    SELECT access_type,executable,process_name,minimum_loader_version
      INTO v_access,v_executable,v_process,v_minimum
    FROM games WHERE id=v_game FOR UPDATE;

    SELECT COUNT(*),MIN(id) INTO v_count,v_patch FROM patches WHERE game_id=v_game;
    IF v_count>1 THEN
        SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT='Multiple Shadows patches found; select one before staging';
    END IF;
    IF v_patch IS NULL THEN
        INSERT INTO patches(game_id,name) VALUES(v_game,'Turkce Yama');
        SET v_patch=LAST_INSERT_ID();
    END IF;
    IF EXISTS(SELECT 1 FROM patch_versions WHERE patch_id=v_patch
              AND version='1.0.0-shadows.1' AND channel='internal') THEN
        SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT='Shadows internal draft already exists; nothing overwritten';
    END IF;

    -- Game detection is shared by every version. Never change it under a
    -- published release; an incompatible existing profile needs manual review.
    IF EXISTS(SELECT 1 FROM patch_versions WHERE patch_id=v_patch AND status='PUBLISHED') THEN
        SELECT COUNT(DISTINCT rule_value) INTO v_count FROM game_detection_rules
        WHERE game_id=v_game AND rule_type='required_file' AND is_required=1
          AND BINARY rule_value IN ('DataPC_boot.forge','DataPC_boot_patch_01.forge',
            'DataPC_boot_patch_02.forge','dlc_20/DataPC_boot_20_dlc.forge',
            'dlc_20/DataPC_boot_20_dlc_patch_01.forge');
        IF NOT (BINARY v_executable <=> BINARY 'ACShadows.exe')
           OR NOT (BINARY v_process <=> BINARY 'ACShadows.exe') OR v_count<>5 THEN
            SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT='Published Shadows detection differs; no changes applied';
        END IF;
    ELSE
        IF v_minimum NOT REGEXP '^[0-9]+[.][0-9]+[.][0-9]+$' THEN
            SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT='Existing game minimum Loader version needs manual review';
        END IF;
        IF CAST(SUBSTRING_INDEX(v_minimum,'.',1) AS UNSIGNED)=0
           AND (CAST(SUBSTRING_INDEX(SUBSTRING_INDEX(v_minimum,'.',2),'.',-1) AS UNSIGNED)=0
             OR (CAST(SUBSTRING_INDEX(SUBSTRING_INDEX(v_minimum,'.',2),'.',-1) AS UNSIGNED)=1
                 AND CAST(SUBSTRING_INDEX(v_minimum,'.',-1) AS UNSIGNED)<5)) THEN
            SET v_minimum='0.1.5';
        END IF;
        UPDATE games SET executable='ACShadows.exe',process_name='ACShadows.exe',
                         minimum_loader_version=v_minimum WHERE id=v_game;
        INSERT INTO game_detection_rules(game_id,provider,rule_type,rule_value,sort_order,is_required)
        SELECT v_game,'manual','required_file',f.path,f.position,1 FROM (
            SELECT 'DataPC_boot.forge' AS path,10 AS position
            UNION ALL SELECT 'DataPC_boot_patch_01.forge',20
            UNION ALL SELECT 'DataPC_boot_patch_02.forge',30
            UNION ALL SELECT 'dlc_20/DataPC_boot_20_dlc.forge',40
            UNION ALL SELECT 'dlc_20/DataPC_boot_20_dlc_patch_01.forge',50
        ) AS f
        WHERE NOT EXISTS(SELECT 1 FROM game_detection_rules r WHERE r.game_id=v_game
            AND r.rule_type='required_file' AND r.rule_value=f.path AND r.is_required=1);
    END IF;

    INSERT INTO patch_versions(patch_id,version,changelog,minimum_loader_version,
                               status,channel,mandatory_update,access_type,schema_version)
    VALUES(v_patch,'1.0.0-shadows.1',
           'ORIJINAL Shadows: 82 ceviri kaydi, bes FORGE; gercek oyun testi bekleniyor.',
           '0.1.5','DRAFT','internal',0,v_access,1);
    SET v_version=LAST_INSERT_ID();
    INSERT INTO patch_archives(patch_version_id,source_type,external_url,storage_name,
                               original_name,mime_type,sha256,size_bytes,file_tree)
    VALUES(v_version,'external',
           'https://www.mediafire.com/file/pi789nzgd1gyle1/ANIMUS_AC_SHADOWS.zip/file',
           CONCAT('external-shadows-',v_version,'-',REPLACE(UUID(),'-',''),'.zip'),
           'ANIMUS_AC_SHADOWS.zip','application/zip',
           '931fd60132db50320a7db4bc815a371d752352a7acdb4e57ee1153d65899aed0',
           83350413,JSON_ARRAY());
    INSERT INTO patch_install_actions(patch_version_id,action_uuid,action_type,source_path,
                                     destination_path,backup_enabled,expected_sha256,
                                     sort_order,options_json)
    VALUES(v_version,'762b942d-58d8-4524-beb7-ae37cce1a6d7','INSTALL_SHADOWS',
           'MANIFEST_SHA256.txt','DataPC_boot.forge',1,
           'ee397ca77e96e0a1c53fff3cf0d26cd58b64a99cdad8c69d9ef34014d87ff6e4',
           10,JSON_OBJECT());
    COMMIT;
    SELECT v_game AS game_id,v_patch AS patch_id,v_version AS staged_version_id,
           'DRAFT' AS status,'internal' AS channel,
           'Review and publish via admin; activate a newly created game for testing' AS next_step;
END$$
DELIMITER ;
CALL animus_stage_shadows_mediafire();
DROP PROCEDURE IF EXISTS animus_stage_shadows_mediafire;
