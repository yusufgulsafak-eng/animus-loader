use crate::error::{LoaderError, Result};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub fn data_root() -> Result<PathBuf> {
    let root = dirs::data_local_dir()
        .ok_or_else(|| LoaderError::Other("Local app data bulunamadı".into()))?
        .join("AnimusPatchLoader");
    fs::create_dir_all(&root)?;
    Ok(root)
}

fn backups_root() -> Result<PathBuf> {
    Ok(data_root()?.join("backups"))
}

fn recovery_backups_root() -> Result<PathBuf> {
    Ok(data_root()?.join("recovery").join("backups"))
}

/// LocalAppData/backups altındaki bir dosyanın ikinci kurtarma kopyası.
/// Kullanıcı normal backup klasörünü yanlışlıkla silse bile bu alan bağımsız
/// kaldığı için kaldırma sırasında dosya geri alınabilir.
fn recovery_path_for_backup(path: &Path) -> Result<Option<PathBuf>> {
    let backups = backups_root()?;
    let Ok(relative) = path.strip_prefix(&backups) else {
        return Ok(None);
    };

    // Sadece gerçek backup alt yollarını kabul et. `..` gibi bileşenlere izin
    // verme; Path::strip_prefix sonrası bile fail-closed kalalım.
    if relative.as_os_str().is_empty()
        || relative.components().any(|component| {
            !matches!(
                component,
                std::path::Component::Normal(_)
            )
        })
    {
        return Ok(None);
    }

    Ok(Some(recovery_backups_root()?.join(relative)))
}

fn copy_atomic(source: &Path, destination: &Path) -> Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = destination.with_extension(format!("animus-tmp-{}", uuid::Uuid::new_v4()));
    fs::copy(source, &temporary)?;
    if destination.exists() {
        fs::remove_file(destination)?;
    }
    fs::rename(temporary, destination)?;
    Ok(())
}

pub fn hash_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 256 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
    {
        let mut file = File::create(&temporary)?;
        serde_json::to_writer_pretty(&mut file, value)?;
        file.flush()?;
        file.sync_all()?;
    }
    fs::rename(temporary, path)?;
    Ok(())
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    Ok(serde_json::from_reader(File::open(path)?)?)
}

pub fn directory_size(path: &Path) -> u64 {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}

/// Dosya kopyalama aynı zamanda backup dosyaları için bağımsız kurtarma
/// aynası tutar. Kaynak normal backup klasöründen silinmişse otomatik olarak
/// recovery aynasına düşer. Böylece ham Windows `os error 3` kullanıcıya
/// sızmaz ve mümkün olduğunda kaldırma kendini onarır.
pub fn copy_file(source: &Path, destination: &Path) -> Result<()> {
    let mut actual_source = source.to_path_buf();
    let source_missing = !actual_source.is_file();

    if source_missing {
        if let Some(recovery) = recovery_path_for_backup(source)? {
            if recovery.is_file() {
                actual_source = recovery;
            } else {
                return Err(LoaderError::Conflict(format!(
                    "Geri yükleme yedeği bulunamadı: {}. Kurtarma kopyası da mevcut değil.",
                    source.display()
                )));
            }
        } else {
            return Err(LoaderError::Other(format!(
                "Kaynak dosya bulunamadı: {}",
                source.display()
            )));
        }
    }

    copy_atomic(&actual_source, destination)?;

    // Yeni bir backup dosyası oluşturuluyorsa ikinci, bağımsız kopyayı da tut.
    // Restore sırasında destination oyun dosyası olduğu için bu blok çalışmaz.
    if let Some(recovery) = recovery_path_for_backup(destination)? {
        if actual_source != recovery {
            copy_atomic(&actual_source, &recovery)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_copy_still_works() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.bin");
        let destination = temp.path().join("nested/destination.bin");
        fs::write(&source, b"ANIMUS").unwrap();
        copy_file(&source, &destination).unwrap();
        assert_eq!(fs::read(destination).unwrap(), b"ANIMUS");
    }
}
