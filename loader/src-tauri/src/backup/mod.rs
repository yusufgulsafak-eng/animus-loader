use crate::{
    error::{LoaderError, Result},
    models::{BackupInfo, Installation, InstallationSummary, PruneReport},
    storage::{copy_file, data_root, directory_size, read_json, recovery_backups_root, write_json_atomic},
};
use std::{collections::HashSet, fs, path::{Path, PathBuf}};

fn valid_backup_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn raw_backup_root(id: &str) -> Result<PathBuf> {
    if !valid_backup_id(id) {
        return Err(LoaderError::UnsafePath(id.into()));
    }
    Ok(data_root()?.join("backups").join(id))
}

fn recovery_backup_root(id: &str) -> Result<PathBuf> {
    if !valid_backup_id(id) {
        return Err(LoaderError::UnsafePath(id.into()));
    }
    Ok(recovery_backups_root()?.join(id))
}

fn installation_references_backup(id: &str) -> Result<bool> {
    let dir = data_root()?.join("installations");
    if !dir.is_dir() {
        return Ok(false);
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(install) = read_json::<Installation>(&path) {
            if install.backup_id == id {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn restore_tree(source: &Path, destination: &Path) -> Result<()> {
    if !source.is_dir() {
        return Err(LoaderError::Conflict(format!(
            "Kurtarma yedeği bulunamadı: {}",
            source.display()
        )));
    }
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err(LoaderError::UnsafePath(entry.path().display().to_string()));
        }
        let target = destination.join(entry.file_name());
        if kind.is_dir() {
            restore_tree(&entry.path(), &target)?;
        } else if kind.is_file() {
            copy_file(&entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Backup klasörü yanlışlıkla silinmişse bağımsız recovery aynasından primary
/// backup'ı otomatik yeniden oluşturur. Recovery de yoksa ve bu id aktif bir
/// installation kaydına aitse fail-closed davranır; force uninstall bile oyun
/// dosyalarına dokunmadan önce burada durur.
pub fn backup_root(id: &str) -> Result<PathBuf> {
    let primary = raw_backup_root(id)?;
    if primary.join("metadata.json").is_file() {
        return Ok(primary);
    }

    let recovery = recovery_backup_root(id)?;
    if recovery.join("metadata.json").is_file() {
        if primary.exists() {
            fs::remove_dir_all(&primary)?;
        }
        restore_tree(&recovery, &primary)?;
        return Ok(primary);
    }

    if installation_references_backup(id)? {
        return Err(LoaderError::Conflict(
            "Kurulumun yedeği ve kurtarma kopyası bulunamadı; güvenlik için oyun dosyalarına dokunulmadı. Oyun dosyalarını Steam/Ubisoft üzerinden doğrulayıp yamayı yeniden kurun.".into(),
        ));
    }

    // Yeni kurulumlarda backup_id önce üretilir, klasör hemen ardından oluşturulur.
    Ok(primary)
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
        let primary = raw_backup_root(&install.backup_id)?;
        let recovery = recovery_backup_root(&install.backup_id)?;
        let backup_exists = primary.join("metadata.json").is_file()
            || recovery.join("metadata.json").is_file();
        result.push(InstallationSummary {
            game_id: install.game_id,
            game_name: install.game_name,
            patch_id: install.patch_id,
            patch_version: install.patch_version,
            root_exists: PathBuf::from(&install.game_root).is_dir(),
            game_root: install.game_root,
            backup_exists,
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
    // Başarılı rollback sonrası recovery varsa primary backup burada otomatik
    // rehydrate edilir ve inactive metadata iki kopyaya da yazılır.
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

/// Aktif hicbir kuruluma bagli olmayan primary + recovery backup klasorlerini
/// ve indirme onbellegini temizler.
pub fn prune() -> Result<PruneReport> {
    let mut active = HashSet::new();
    for summary in list_installations()? {
        active.insert(summary.backup_id);
    }

    let mut removed = 0u64;
    let mut freed = 0u64;
    for root in [data_root()?.join("backups"), recovery_backups_root()?] {
        if !root.is_dir() {
            continue;
        }
        for entry in fs::read_dir(&root)? {
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
    if !valid_backup_id(id) {
        return Err(LoaderError::UnsafePath(id.into()));
    }
    if installation_references_backup(id)? {
        return Err(LoaderError::Conflict(
            "Aktif kurulumun zorunlu backup/kurtarma kopyası silinemez".into(),
        ));
    }

    let primary = raw_backup_root(id)?;
    let recovery = recovery_backup_root(id)?;

    let metadata = if primary.join("metadata.json").is_file() {
        primary.join("metadata.json")
    } else {
        recovery.join("metadata.json")
    };
    if metadata.is_file() {
        let install: Installation = read_json(&metadata)?;
        if install.active {
            return Err(LoaderError::Conflict(
                "Aktif kurulumun zorunlu backup'ı silinemez".into(),
            ));
        }
    }

    if primary.is_dir() {
        fs::remove_dir_all(&primary)?;
    }
    if recovery.is_dir() {
        fs::remove_dir_all(&recovery)?;
    }
    Ok(())
}
