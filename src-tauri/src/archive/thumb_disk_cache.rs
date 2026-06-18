use super::extract_cache;
use crate::error::{AppError, AppResult};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

fn thumb_root() -> PathBuf {
    extract_cache::cache_root()
        .parent()
        .map(|p| p.join("thumb-cache"))
        .unwrap_or_else(|| extract_cache::cache_root().join("thumb-cache"))
}

fn entry_key(entry_path: &str) -> String {
    let norm = extract_cache::normalize_entry_path(entry_path);
    let mut hasher = Sha256::new();
    hasher.update(norm.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn thumb_path(archive_hash: &str, entry_path: &str) -> PathBuf {
    thumb_root()
        .join(archive_hash)
        .join(format!("{}.webp", entry_key(entry_path)))
}

pub fn read_if_exists(archive_hash: &str, entry_path: &str) -> Option<Vec<u8>> {
    let path = thumb_path(archive_hash, entry_path);
    fs::read(&path).ok()
}

pub fn write(archive_hash: &str, entry_path: &str, bytes: &[u8]) -> AppResult<()> {
    let dest = thumb_path(archive_hash, entry_path);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::Archive(e.to_string()))?;
    }
    let tmp = dest.with_extension("webp.part");
    fs::write(&tmp, bytes).map_err(|e| AppError::Archive(e.to_string()))?;
    if dest.exists() {
        fs::remove_file(&dest).ok();
    }
    fs::rename(&tmp, &dest).map_err(|e| AppError::Archive(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn thumb_roundtrip() {
        let dir = TempDir::new().unwrap();
        extract_cache::set_custom_root(Some(
            dir.path().join("extract").to_string_lossy().into_owned(),
        ));
        let hash = "abc123";
        let data = b"fake-webp";
        write(hash, "img/photo.png", data).unwrap();
        let hit = read_if_exists(hash, "img/photo.png");
        assert_eq!(hit.as_deref(), Some(data.as_slice()));
        extract_cache::set_custom_root(None);
    }
}