use crate::error::{AppError, AppResult};
use camino::{Utf8Path, Utf8PathBuf};
use std::path::{Component, Path, PathBuf};

const WIN_RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// `dest` — абсолютный canonical root; `entry_path` — путь внутри архива.
pub fn resolve_safe_extract_path(dest: &Path, entry_path: &str) -> AppResult<PathBuf> {
    if entry_path.contains('\0') {
        return Err(AppError::ArchiveZipSlip {
            entry: entry_path.to_string(),
            reason: "null byte in path".into(),
        });
    }

    let normalized = normalize_archive_entry_path(entry_path)?;
    validate_components(&normalized)?;

    let dest_root = canonical_dest_root(dest)?;
    let joined = dest_root.join(normalized.as_str());

    ensure_within_root(&dest_root, &joined, entry_path)?;
    Ok(joined)
}

fn normalize_archive_entry_path(entry_path: &str) -> AppResult<Utf8PathBuf> {
    let trimmed = entry_path.trim();
    if trimmed.is_empty() {
        return Err(AppError::ArchiveZipSlip {
            entry: entry_path.to_string(),
            reason: "empty entry path".into(),
        });
    }

    let with_slashes = trimmed.replace('\\', "/");
    let mut parts: Vec<&str> = Vec::new();

    for segment in with_slashes.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            return Err(AppError::ArchiveZipSlip {
                entry: entry_path.to_string(),
                reason: "parent traversal (..)".into(),
            });
        }
        parts.push(segment);
    }

    if parts.is_empty() {
        return Err(AppError::ArchiveZipSlip {
            entry: entry_path.to_string(),
            reason: "path resolves to root".into(),
        });
    }

    let mut out = Utf8PathBuf::new();
    for part in parts {
        out.push(part);
    }
    Ok(out)
}

fn validate_components(path: &Utf8Path) -> AppResult<()> {
    for component in path.components() {
        let camino::Utf8Component::Normal(name) = component else {
            return Err(AppError::ArchiveZipSlip {
                entry: path.as_str().to_string(),
                reason: "invalid path component".into(),
            });
        };

        if name.starts_with('/') || name.contains(':') {
            return Err(AppError::ArchiveZipSlip {
                entry: path.as_str().to_string(),
                reason: "absolute path segment".into(),
            });
        }

        if has_trailing_dots_or_spaces(name) {
            return Err(AppError::ArchiveZipSlip {
                entry: path.as_str().to_string(),
                reason: "trailing dots or spaces (Windows)".into(),
            });
        }

        let stem = strip_extension(name);
        if is_reserved_name(stem) {
            return Err(AppError::ArchiveReservedName(name.to_string()));
        }
    }
    Ok(())
}

fn canonical_dest_root(dest: &Path) -> AppResult<PathBuf> {
    if dest.is_absolute() {
        if let Ok(canon) = dest.canonicalize() {
            return Ok(canon);
        }
        std::fs::create_dir_all(dest)?;
        return dest.canonicalize().map_err(|e| AppError::Archive(e.to_string()));
    }

    let abs = std::env::current_dir()
        .map_err(|e| AppError::Archive(e.to_string()))?
        .join(dest);
    if abs.exists() {
        return abs
            .canonicalize()
            .map_err(|e| AppError::Archive(e.to_string()));
    }
    std::fs::create_dir_all(&abs)?;
    abs.canonicalize()
        .map_err(|e| AppError::Archive(e.to_string()))
}

fn ensure_within_root(root: &Path, candidate: &Path, entry: &str) -> AppResult<()> {
    let root_norm = normalize_for_prefix(root);
    let candidate_norm = normalize_for_prefix(candidate);

    if !candidate_norm.starts_with(&root_norm) {
        return Err(AppError::ArchiveZipSlip {
            entry: entry.to_string(),
            reason: "path escapes destination root".into(),
        });
    }

    let rel = candidate_norm
        .strip_prefix(&root_norm)
        .unwrap_or(candidate);

    for component in Path::new(rel).components() {
        if matches!(component, Component::ParentDir) {
            return Err(AppError::ArchiveZipSlip {
                entry: entry.to_string(),
                reason: "path escapes destination root".into(),
            });
        }
    }
    Ok(())
}

fn normalize_for_prefix(path: &Path) -> PathBuf {
    path.components().collect()
}

fn has_trailing_dots_or_spaces(name: &str) -> bool {
    name.ends_with('.') || name.ends_with(' ') || name.ends_with('\t')
}

fn strip_extension(name: &str) -> &str {
    name.rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(name)
}

fn is_reserved_name(stem: &str) -> bool {
    let upper = stem.to_ascii_uppercase();
    WIN_RESERVED.iter().any(|r| *r == upper)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn dest_root(dir: &TempDir) -> PathBuf {
        dir.path().canonicalize().unwrap()
    }

    #[test]
    fn rejects_parent_traversal() {
        let dir = TempDir::new().unwrap();
        let dest = dest_root(&dir);
        assert!(resolve_safe_extract_path(&dest, "../../../etc/passwd").is_err());
        assert!(resolve_safe_extract_path(&dest, "subdir/../../../evil.exe").is_err());
    }

    #[test]
    fn rejects_absolute_windows_path() {
        let dir = TempDir::new().unwrap();
        let dest = dest_root(&dir);
        assert!(resolve_safe_extract_path(&dest, r"C:\Windows\System32\calc.exe").is_err());
    }

    #[test]
    fn rejects_reserved_con() {
        let dir = TempDir::new().unwrap();
        let dest = dest_root(&dir);
        let err = resolve_safe_extract_path(&dest, "CON.txt").unwrap_err();
        assert!(matches!(err, AppError::ArchiveReservedName(_)));
    }

    #[test]
    fn accepts_normal_path() {
        let dir = TempDir::new().unwrap();
        let dest = dest_root(&dir);
        let out = resolve_safe_extract_path(&dest, "normal/part.stl").unwrap();
        assert!(out.ends_with("normal/part.stl"));
        assert!(out.starts_with(&dest));
    }

    #[test]
    fn rejects_null_byte() {
        let dir = TempDir::new().unwrap();
        let dest = dest_root(&dir);
        assert!(resolve_safe_extract_path(&dest, "a\0b.stl").is_err());
    }

    #[test]
    fn written_file_stays_in_dest() {
        let dir = TempDir::new().unwrap();
        let dest = dest_root(&dir);
        let out = resolve_safe_extract_path(&dest, "folder/file.stl").unwrap();
        fs::create_dir_all(out.parent().unwrap()).unwrap();
        fs::write(&out, b"ok").unwrap();
        assert!(out.starts_with(&dest));
    }
}
