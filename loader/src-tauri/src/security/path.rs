use crate::error::{LoaderError, Result};
use std::path::{Component, Path, PathBuf};

pub fn validate_relative(value: &str) -> Result<PathBuf> {
    if value.trim().is_empty()
        || value.contains('\0')
        || value.starts_with("\\\\")
        || value.starts_with("//")
    {
        return Err(LoaderError::UnsafePath(value.into()));
    }
    let path = Path::new(value);
    if path.is_absolute() {
        return Err(LoaderError::UnsafePath(value.into()));
    }
    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            _ => return Err(LoaderError::UnsafePath(value.into())),
        }
    }
    Ok(path.to_path_buf())
}

pub fn resolve_inside(root: &Path, relative: &str) -> Result<PathBuf> {
    let rel = validate_relative(relative)?;
    let canonical_root = root
        .canonicalize()
        .map_err(|_| LoaderError::UnsafePath(root.display().to_string()))?;
    let candidate = canonical_root.join(rel);
    let mut ancestor = candidate.as_path();
    while !ancestor.exists() {
        ancestor = ancestor
            .parent()
            .ok_or_else(|| LoaderError::UnsafePath(relative.into()))?;
    }
    let real_ancestor = ancestor.canonicalize()?;
    if !real_ancestor.starts_with(&canonical_root) {
        return Err(LoaderError::UnsafePath(relative.into()));
    }
    if candidate.exists() && !candidate.canonicalize()?.starts_with(&canonical_root) {
        return Err(LoaderError::UnsafePath(relative.into()));
    }
    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_escape() {
        assert!(validate_relative("../Windows").is_err());
        assert!(validate_relative("C:\\Windows").is_err());
        assert!(validate_relative("\\\\server\\share").is_err());
    }
    #[test]
    fn accepts_nested() {
        assert!(validate_relative("Content/Paks/file.pak").is_ok());
    }
}
