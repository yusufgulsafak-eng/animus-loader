use crate::error::{LoaderError, Result};
use crate::security::path::validate_relative;
use std::{
    fs::{self, File},
    io,
    path::Path,
};
use zip::ZipArchive;

const MAX_FILES: usize = 100_000;
const MAX_UNCOMPRESSED: u64 = 20 * 1024 * 1024 * 1024;

pub fn extract_safe(archive: &Path, target: &Path) -> Result<()> {
    fs::create_dir_all(target)?;
    let mut zip = ZipArchive::new(File::open(archive)?)?;
    if zip.len() > MAX_FILES {
        return Err(LoaderError::Manifest(
            "ZIP dosya sayısı sınırı aşıldı".into(),
        ));
    }
    let mut total = 0u64;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        let raw = entry.name().replace('\\', "/");
        let relative = validate_relative(raw.trim_end_matches('/'))?;
        if entry
            .unix_mode()
            .map(|m| m & 0o170000 == 0o120000)
            .unwrap_or(false)
        {
            return Err(LoaderError::UnsafePath(format!("ZIP symlink: {raw}")));
        }
        total = total
            .checked_add(entry.size())
            .ok_or_else(|| LoaderError::Manifest("ZIP boyutu taştı".into()))?;
        if total > MAX_UNCOMPRESSED {
            return Err(LoaderError::Manifest(
                "ZIP açılmış boyut sınırı aşıldı".into(),
            ));
        }
        let output = target.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&output)?;
            continue;
        }
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = File::create(&output)?;
        io::copy(&mut entry, &mut file)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn rejects_zip_slip_entry() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("unsafe.zip");
        let mut writer = zip::ZipWriter::new(File::create(&archive).unwrap());
        writer
            .start_file("../outside.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"blocked").unwrap();
        writer.finish().unwrap();
        let target = temp.path().join("extract");
        assert!(extract_safe(&archive, &target).is_err());
        assert!(!temp.path().join("outside.txt").exists());
    }
}
