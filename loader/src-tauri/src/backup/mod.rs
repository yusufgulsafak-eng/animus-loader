use crate::{
    error::{LoaderError, Result},
    models::{BackupInfo, Installation},
    storage::{data_root, directory_size, read_json},
};
use std::{fs, path::PathBuf};
pub fn backup_root(id: &str) -> Result<PathBuf> {
    Ok(data_root()?.join("backups").join(id))
}
pub fn installation_path(game_id: u64) -> Result<PathBuf> {
    Ok(data_root()?
        .join("installations")
        .join(format!("{game_id}.json")))
}
pub fn list() -> Result<Vec<BackupInfo>> {
    let root = data_root()?.join("backups");
    if !root.exists() {
        return Ok(vec![]);
    }
    let mut result = vec![];
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        let metadata = path.join("metadata.json");
        if !metadata.is_file() {
            continue;
        }
        let install: Installation = read_json(&metadata)?;
        result.push(BackupInfo {
            id: install.backup_id,
            game_name: install.game_name,
            version: install.patch_version,
            created_at: install.created_at,
            size_bytes: directory_size(&path),
            active: install.active,
        });
    }
    result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(result)
}
pub fn clean(id: &str) -> Result<()> {
    if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(LoaderError::UnsafePath(id.into()));
    }
    let root = backup_root(id)?;
    let install: Installation = read_json(&root.join("metadata.json"))?;
    if install.active {
        return Err(LoaderError::Conflict(
            "Aktif kurulumun zorunlu backup'ı silinemez".into(),
        ));
    }
    fs::remove_dir_all(root)?;
    Ok(())
}
