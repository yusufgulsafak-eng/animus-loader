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

/// Dosyanın SHA-256 değerini hesaplar.
///
/// ÖNEMLİ:
/// Büyük buffer stack üzerinde tutulmaz.
/// Heap üzerinde Vec kullanılır. Böylece Windows worker thread'lerinde
/// stack overflow nedeniyle uygulamanın kapanma ihtimali ortadan kaldırılır.
pub fn hash_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();

    // 256 KB heap buffer.
    // Önceki 1 MB stack buffer yerine güvenli heap allocation kullanıyoruz.
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

/// JSON dosyasını mümkün olduğunca güvenli şekilde yazar.
///
/// Windows std::fs::rename mevcut hedef dosyayı doğrudan ezemeyebilir.
/// Bu nedenle:
/// 1. Önce temporary dosya yazılır.
/// 2. flush + sync yapılır.
/// 3. Eski hedef varsa kaldırılır.
/// 4. Temporary dosya gerçek hedefe taşınır.
pub fn write_json_atomic<T: Serialize>(
    path: &Path,
    value: &T,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let temporary = path.with_extension(format!(
        "tmp-{}",
        uuid::Uuid::new_v4()
    ));

    let write_result = (|| -> Result<()> {
        {
            let mut file = File::create(&temporary)?;

            serde_json::to_writer_pretty(
                &mut file,
                value,
            )?;

            file.flush()?;
            file.sync_all()?;
        }

        // Windows'ta rename mevcut dosyanın üzerine yazamayabilir.
        if path.exists() {
            fs::remove_file(path)?;
        }

        fs::rename(
            &temporary,
            path,
        )?;

        Ok(())
    })();

    // Yazma sırasında hata olduysa geçici dosya bırakma.
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary);
    }

    write_result
}

pub fn read_json<T: DeserializeOwned>(
    path: &Path,
) -> Result<T> {
    let file = File::open(path)?;

    Ok(
        serde_json::from_reader(file)?
    )
}

pub fn directory_size(
    path: &Path,
) -> u64 {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.metadata().ok())
        .filter(|metadata| metadata.is_file())
        .map(|metadata| metadata.len())
        .sum()
}

/// Dosyayı güvenli biçimde kopyalar.
///
/// Önce destination ile aynı klasörde geçici dosya oluşturulur.
/// Kopyalama başarıyla tamamlandıktan sonra gerçek hedef değiştirilir.
pub fn copy_file(
    source: &Path,
    destination: &Path,
) -> Result<()> {
    if !source.is_file() {
        return Err(
            LoaderError::Other(format!(
                "Kaynak dosya bulunamadı: {}",
                source.display()
            ))
        );
    }

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    let temporary = destination.with_extension(format!(
        "animus-tmp-{}",
        uuid::Uuid::new_v4()
    ));

    let copy_result = (|| -> Result<()> {
        fs::copy(
            source,
            &temporary,
        )?;

        // Geçici dosyanın gerçekten oluştuğunu doğrula.
        if !temporary.is_file() {
            return Err(
                LoaderError::Other(
                    "Geçici dosya oluşturulamadı.".into()
                )
            );
        }

        if destination.exists() {
            if destination.is_dir() {
                return Err(
                    LoaderError::Other(format!(
                        "Hedef bir klasör: {}",
                        destination.display()
                    ))
                );
            }

            fs::remove_file(destination)?;
        }

        fs::rename(
            &temporary,
            destination,
        )?;

        Ok(())
    })();

    // Hata durumunda geçici dosya bırakma.
    if copy_result.is_err() {
        let _ = fs::remove_file(&temporary);
    }

    copy_result
}
