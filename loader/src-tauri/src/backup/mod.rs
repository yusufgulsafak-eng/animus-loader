use crate::{
    error::{LoaderError, Result},
    models::{BackupInfo, Installation, InstallationSummary, PruneReport},
    storage::{data_root, directory_size, read_json, write_json_atomic},
};
use std::{collections::HashSet, fs, path::PathBuf};
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
pub fn installations_dir() -> Result<PathBuf> {
    let dir = data_root()?.join("installations");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Diskteki tum aktif kurulum kayitlari. Tek dogruluk kaynagi budur.
pub fn list_installations() -> Result<Vec<InstallationSummary>> {
    let dir = installations_dir()?;
    let mut result = vec![];
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        // Bozuk tek bir kayit tum listeyi dusurmemeli.
        let Ok(install) = read_json::<Installation>(&path) else {
            continue;
        };
        let backup = backup_root(&install.backup_id)?;
        result.push(InstallationSummary {
            game_id: install.game_id,
            game_name: install.game_name,
            patch_id: install.patch_id,
            patch_version: install.patch_version,
            root_exists: PathBuf::from(&install.game_root).is_dir(),
            game_root: install.game_root,
            backup_exists: backup.join("metadata.json").is_file(),
            backup_id: install.backup_id,
            created_at: install.created_at,
            change_count: install.changes.len() as u64,
        });
    }
    result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(result)
}

pub fn find_installation(game_id: u64) -> Result<Option<Installation>> {
    let path = installation_path(game_id)?;
    if !path.is_file() {
        return Ok(None);
    }
    match read_json::<Installation>(&path) {
        Ok(install) => Ok(Some(install)),
        // Okunamayan kayit "kurulum yok" sayilir; kullanici yeniden kurabilsin.
        Err(_) => Ok(None),
    }
}

pub fn close_installation(install: &Installation) -> Result<()> {
    let backup = backup_root(&install.backup_id)?;
    if backup.join("metadata.json").is_file() {
        let mut closed = install.clone();
        closed.active = false;
        write_json_atomic(&backup.join("metadata.json"), &closed)?;
    }
    let path = installation_path(install.game_id)?;
    if path.is_file() {
        fs::remove_file(path)?;
    }
    Ok(())
}

/// Aktif hicbir kuruluma bagli olmayan backup klasorlerini ve indirme
/// onbellegini temizler. Loader surekli calistikca disk sismesin diye gerekli.
pub fn prune() -> Result<PruneReport> {
    let mut active = HashSet::new();
    for summary in list_installations()? {
        active.insert(summary.backup_id);
    }
    let root = data_root()?.join("backups");
    let mut removed = 0u64;
    let mut freed = 0u64;
    if root.is_dir() {
        for entry in fs::read_dir(root)? {
            let path = entry?.path();
            if !path.is_dir() {
                continue;
            }
            let id = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            if active.contains(&id) {
                continue;
            }
            let size = directory_size(&path);
            if fs::remove_dir_all(&path).is_ok() {
                removed += 1;
                freed += size;
            }
        }
    }
    let cache_bytes = crate::download::clear_cache()?;
    Ok(PruneReport {
        removed_backups: removed,
        freed_bytes: freed,
        cache_bytes,
    })
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
