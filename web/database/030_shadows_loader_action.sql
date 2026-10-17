-- Adds an action type without publishing or modifying a game/patch version.
-- Keep all currently installed enum values, including RUN_BAT.
DROP PROCEDURE IF EXISTS animus_add_shadows_action;
DELIMITER //
CREATE PROCEDURE animus_add_shadows_action()
BEGIN
  DECLARE current_type TEXT;
  SELECT COLUMN_TYPE INTO current_type FROM information_schema.COLUMNS
    WHERE TABLE_SCHEMA=DATABASE() AND TABLE_NAME='patch_install_actions'
      AND COLUMN_NAME='action_type';
  IF current_type IS NULL OR LEFT(current_type,5) <> 'enum(' THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT='patch_install_actions.action_type enum not found';
  END IF;
  IF LOCATE('''INSTALL_SHADOWS''', current_type)=0 THEN
    SET @shadows_sql=CONCAT('ALTER TABLE patch_install_actions MODIFY COLUMN action_type ',
      LEFT(current_type,CHAR_LENGTH(current_type)-1), ',''INSTALL_SHADOWS'') NOT NULL');
    PREPARE shadows_statement FROM @shadows_sql;
    EXECUTE shadows_statement;
    DEALLOCATE PREPARE shadows_statement;
  END IF;
END//
DELIMITER ;
CALL animus_add_shadows_action();
DROP PROCEDURE animus_add_shadows_action;
