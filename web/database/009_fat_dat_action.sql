-- Add native small archive updates; does not publish or change any game.
ALTER TABLE patch_install_actions MODIFY COLUMN action_type
ENUM('COPY_FILE','COPY_DIRECTORY','REPLACE_FILE','DELETE_FILE','DELETE_DIRECTORY','CREATE_DIRECTORY','MOVE_FILE','RENAME_FILE','APPEND_FAT_DAT') NOT NULL;


