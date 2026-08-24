ALTER TABLE games
  ADD COLUMN icon_url VARCHAR(1000) NULL AFTER banner_url,
  ADD COLUMN local_icon_path VARCHAR(500) NULL AFTER local_banner_path,
  ADD COLUMN icon_path VARCHAR(500) NULL AFTER banner_path;

UPDATE games SET icon_path='/assets/placeholders/icon-generic.svg' WHERE icon_path IS NULL OR icon_path='';
