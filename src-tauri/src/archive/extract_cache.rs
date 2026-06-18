use crate::error::{AppError, AppResult};
use lru::LruCache;
use sha2::{Digest, Sha256};
use std::fs;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_CACHE_BYTES: u64 = 5 * 1024 * 1024 * 1024;
const MAX_LRU_ENTRIES: usize = 4096;

struct LruState {
    total_bytes: u64,
    entries: LruCache<PathBuf, u64>,
}

static CUSTOM_ROOT: OnceLock<Mutex<Option<String>>> = OnceLock::new();
static LRU: OnceLock<Mutex<LruState>> = OnceLock::new();

fn lru_state() -> &'static Mutex<LruState> {
    LRU.get_or_init(|| {
        let cap = NonZeroUsize::new(MAX_LRU_ENTRIES).unwrap_or(NonZeroUsize::MIN);
        Mutex::new(LruState {
            total_bytes: 0,
            entries: LruCache::new(cap),
        })
    })
}

pub fn set_custom_root(path: Option<String>) {
    let guard = CUSTOM_ROOT.get_or_init(|| Mutex::new(None));
    if let Ok(mut g) = guard.lock() {
        *g = path.filter(|p| !p.trim().is_empty());
    }
}

fn custom_root() -> Option<String> {
    CUSTOM_ROOT
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()
        .and_then(|g| g.clone())
}

pub fn cache_root() -> PathBuf {
    if let Some(custom) = custom_root() {
        return PathBuf::from(custom);
    }
    dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("Hehel-Zip")
        .join("extract-cache")
}

pub fn archive_hash(archive_path: &str) -> AppResult<String> {
    let meta = fs::metadata(archive_path).map_err(|e| AppError::Archive(e.to_string()))?;
    let mtime = meta
        .modified()
        .unwrap_or(SystemTime::UNIX_EPOCH)
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let size = meta.len();
    let canonical = dunce::canonicalize(archive_path)
        .unwrap_or_else(|_| PathBuf::from(archive_path))
        .to_string_lossy()
        .into_owned();
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    hasher.update(mtime.to_le_bytes());
    hasher.update(size.to_le_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn normalize_entry_path(p: &str) -> String {
    p.replace('\\', "/").trim_end_matches('/').to_string()
}

fn entry_key(entry_path: &str, preserve_paths: bool) -> String {
    let norm = normalize_entry_path(entry_path);
    let mut hasher = Sha256::new();
    hasher.update(norm.as_bytes());
    hasher.update([u8::from(preserve_paths)]);
    format!("{:x}", hasher.finalize())
}

pub fn cache_file_path_with_hash(
    archive_hash: &str,
    entry_path: &str,
    preserve_paths: bool,
) -> PathBuf {
    let ek = entry_key(entry_path, preserve_paths);
    cache_root().join(archive_hash).join(ek)
}

pub fn cache_file_path(
    archive_path: &str,
    entry_path: &str,
    preserve_paths: bool,
) -> AppResult<PathBuf> {
    let ah = archive_hash(archive_path)?;
    Ok(cache_file_path_with_hash(&ah, entry_path, preserve_paths))
}

pub fn part_path_for(dest: &Path) -> PathBuf {
    let file_name = dest
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    let mut part_name = file_name;
    part_name.push(".part");
    dest.parent()
        .map(|p| p.join(part_name))
        .unwrap_or_else(|| dest.with_extension("part"))
}

pub fn prepare_cache_write(
    archive_hash: &str,
    entry_path: &str,
    preserve_paths: bool,
) -> AppResult<(PathBuf, PathBuf)> {
    let dest = cache_file_path_with_hash(archive_hash, entry_path, preserve_paths);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::Archive(e.to_string()))?;
    }
    let part = part_path_for(&dest);
    if part.exists() {
        fs::remove_file(&part).ok();
    }
    Ok((part, dest))
}

pub fn finalize_part(part: &Path, dest: &Path) -> AppResult<PathBuf> {
    if dest.exists() {
        fs::remove_file(dest).ok();
    }
    fs::rename(part, dest).map_err(|e| AppError::Archive(e.to_string()))?;
    let size = fs::metadata(dest).map(|m| m.len()).unwrap_or(0);
    register_and_evict(dest, size);
    Ok(dest.to_path_buf())
}

pub fn get_if_exists_with_hash(
    archive_hash: &str,
    entry_path: &str,
    preserve_paths: bool,
) -> Option<PathBuf> {
    let path = cache_file_path_with_hash(archive_hash, entry_path, preserve_paths);
    if path.is_file() {
        touch_file(&path);
        Some(path)
    } else {
        None
    }
}

pub fn get_if_exists(
    archive_path: &str,
    entry_path: &str,
    preserve_paths: bool,
) -> Option<PathBuf> {
    let path = cache_file_path(archive_path, entry_path, preserve_paths).ok()?;
    if path.is_file() {
        touch_file(&path);
        Some(path)
    } else {
        None
    }
}

pub fn session_out_path(
    session_dir: &Path,
    entry_path: &str,
    preserve_paths: bool,
) -> PathBuf {
    if preserve_paths {
        session_dir.join(entry_path.replace('/', "\\"))
    } else {
        session_dir.join(
            Path::new(entry_path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| entry_path.to_string()),
        )
    }
}

pub fn link_or_copy(cache_path: &Path, session_path: &Path) -> AppResult<()> {
    if let Some(parent) = session_path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::Archive(e.to_string()))?;
    }
    if session_path.exists() {
        fs::remove_file(session_path).ok();
    }
    if fs::hard_link(cache_path, session_path).is_err() {
        fs::copy(cache_path, session_path).map_err(|e| AppError::Archive(e.to_string()))?;
    }
    Ok(())
}

pub fn put_bytes(
    data: &[u8],
    archive_path: &str,
    entry_path: &str,
    preserve_paths: bool,
) -> AppResult<PathBuf> {
    let dest = cache_file_path(archive_path, entry_path, preserve_paths)?;
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::Archive(e.to_string()))?;
    }
    let tmp = dest.with_extension("part");
    fs::write(&tmp, data).map_err(|e| AppError::Archive(e.to_string()))?;
    fs::rename(&tmp, &dest).map_err(|e| AppError::Archive(e.to_string()))?;
    register_and_evict(&dest, data.len() as u64);
    Ok(dest)
}

pub fn put_file(
    src: &Path,
    archive_path: &str,
    entry_path: &str,
    preserve_paths: bool,
) -> AppResult<PathBuf> {
    let dest = cache_file_path(archive_path, entry_path, preserve_paths)?;
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::Archive(e.to_string()))?;
    }
    let tmp = dest.with_extension("part");
    fs::copy(src, &tmp).map_err(|e| AppError::Archive(e.to_string()))?;
    fs::rename(&tmp, &dest).map_err(|e| AppError::Archive(e.to_string()))?;
    let size = fs::metadata(&dest)
        .map(|m| m.len())
        .unwrap_or(0);
    register_and_evict(&dest, size);
    Ok(dest)
}

fn touch_file(path: &Path) {
    let Ok(mut state) = lru_state().lock() else {
        return;
    };
    let path_buf = path.to_path_buf();
    if state.entries.get(&path_buf).is_some() {
        return;
    }
    if let Ok(meta) = fs::metadata(path) {
        register_and_evict_inner(&mut state, &path_buf, meta.len());
    }
}

fn register_and_evict(path: &Path, size: u64) {
    let Ok(mut state) = lru_state().lock() else {
        return;
    };
    register_and_evict_inner(&mut state, &path.to_path_buf(), size);
}

fn register_and_evict_inner(state: &mut LruState, path_buf: &PathBuf, size: u64) {
    if let Some(old_size) = state.entries.pop(path_buf) {
        state.total_bytes = state.total_bytes.saturating_sub(old_size);
    }
    state.total_bytes = state.total_bytes.saturating_add(size);
    state.entries.put(path_buf.clone(), size);
    while state.total_bytes > MAX_CACHE_BYTES {
        let Some((evict_path, evict_size)) = state.entries.pop_lru() else {
            break;
        };
        state.total_bytes = state.total_bytes.saturating_sub(evict_size);
        let _ = fs::remove_file(evict_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn cache_put_and_hit() {
        let dir = TempDir::new().unwrap();
        let archive = dir.path().join("test.hehe");
        fs::write(&archive, b"fake").unwrap();
        set_custom_root(Some(dir.path().join("cache").to_string_lossy().into_owned()));

        let data = b"solid test";
        put_bytes(data, &archive.to_string_lossy(), "part.stl", false).unwrap();
        let hit = get_if_exists(&archive.to_string_lossy(), "part.stl", false);
        assert!(hit.is_some());
        assert_eq!(fs::read(hit.unwrap()).unwrap(), data);

        set_custom_root(None);
    }

    #[test]
    fn archive_hash_changes_with_content() {
        let dir = TempDir::new().unwrap();
        let archive = dir.path().join("a.hehe");
        fs::write(&archive, b"v1").unwrap();
        let h1 = archive_hash(&archive.to_string_lossy()).unwrap();
        fs::write(&archive, b"v2-longer").unwrap();
        let h2 = archive_hash(&archive.to_string_lossy()).unwrap();
        assert_ne!(h1, h2);
    }
}
