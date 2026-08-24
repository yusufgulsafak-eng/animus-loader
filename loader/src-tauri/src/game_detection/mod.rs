use crate::{
    error::{LoaderError, Result},
    security::path::resolve_inside,
};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};
pub fn validate_root(root: &Path, required: &[String]) -> Result<()> {
    if !root.is_dir() {
        return Err(LoaderError::Manifest(
            "Seçilen oyun dizini mevcut değil".into(),
        ));
    }
    for file in required {
        let path = resolve_inside(root, file)?;
        if !path.is_file() {
            return Err(LoaderError::Manifest(format!(
                "Zorunlu oyun dosyası eksik: {file}"
            )));
        }
    }
    Ok(())
}
pub fn detect_steam(app_id: &str, required: &[String]) -> Result<Option<PathBuf>> {
    let mut steam_roots = BTreeSet::new();
    if let Ok(p) = std::env::var("PROGRAMFILES(X86)") {
        steam_roots.insert(PathBuf::from(p).join("Steam"));
    }
    if let Ok(p) = std::env::var("PROGRAMFILES") {
        steam_roots.insert(PathBuf::from(p).join("Steam"));
    }
    if let Some(home) = dirs::home_dir() {
        steam_roots.insert(home.join("AppData/Local/Steam"));
    }
    let mut libraries = BTreeSet::new();
    for steam in steam_roots {
        if !steam.is_dir() {
            continue;
        }
        libraries.insert(steam.clone());
        let vdf = steam.join("steamapps/libraryfolders.vdf");
        if let Ok(text) = fs::read_to_string(vdf) {
            for path in quoted_values(&text, "path") {
                libraries.insert(PathBuf::from(path.replace("\\\\", "\\")));
            }
        }
    }
    for library in libraries {
        let manifest = library
            .join("steamapps")
            .join(format!("appmanifest_{app_id}.acf"));
        if !manifest.is_file() {
            continue;
        }
        let text = fs::read_to_string(manifest)?;
        if let Some(name) = quoted_values(&text, "installdir").into_iter().next() {
            let root = library.join("steamapps/common").join(name);
            if validate_root(&root, required).is_ok() {
                return Ok(Some(root));
            }
        }
    }
    Ok(None)
}
fn quoted_values(text: &str, key: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split('"').filter(|v| !v.trim().is_empty());
            let found = parts.next()?.trim();
            let value = parts.next()?.trim();
            (found.eq_ignore_ascii_case(key)).then(|| value.to_string())
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_vdf() {
        let v = "\"path\" \"D:\\\\SteamLibrary\"\n\"installdir\" \"Demo\"";
        assert_eq!(quoted_values(v, "path")[0], "D:\\\\SteamLibrary");
    }
}
