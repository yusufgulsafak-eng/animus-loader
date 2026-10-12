-- Battlefield V: allow narrowly pinned RUN_BAT manifest action.
-- Deployment order: publish API and Windows Loader 0.1.2 first, then migrate.
ALTER TABLE patch_install_actions MODIFY COLUMN action_type
ENUM('COPY_FILE','COPY_DIRECTORY','REPLACE_FILE','DELETE_FILE',
     'DELETE_DIRECTORY','CREATE_DIRECTORY','MOVE_FILE','RENAME_FILE',
     'APPEND_FAT_DAT','RUN_BAT') NOT NULL;
