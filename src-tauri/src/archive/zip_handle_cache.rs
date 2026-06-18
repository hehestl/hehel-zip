use crate::error::{AppError, AppResult};
use lru::LruCache;
use std::fs::{self, File};
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;
use zip::read::ZipArchive;

const MAX_OPEN_ZIPS: usize = 4;

struct CachedZip {
    mtime: SystemTime,
    size: u64,
    archive: Mutex<ZipArchive<File>>,
}

static CACHE: OnceLock<Mutex<LruCache<String, Arc<CachedZip>>>> = OnceLock::new();

fn cache() -> &'static Mutex<LruCache<String, Arc<CachedZip>>> {
    CACHE.get_or_init(|| {
        let cap = NonZeroUsize::new(MAX_OPEN_ZIPS).unwrap_or(NonZeroUsize::MIN);
        Mutex::new(LruCache::new(cap))
    })
}

fn canonical_key(archive_path: &str) -> AppResult<String> {
    let path = dunce::canonicalize(archive_path)
        .unwrap_or_else(|_| PathBuf::from(archive_path));
    Ok(path.to_string_lossy().into_owned())
}

fn fingerprint(archive_path: &str) -> AppResult<(SystemTime, u64)> {
    let meta = fs::metadata(archive_path).map_err(|e| AppError::Archive(e.to_string()))?;
    Ok((
        meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        meta.len(),
    ))
}

fn open_zip(archive_path: &str, mtime: SystemTime, size: u64) -> AppResult<Arc<CachedZip>> {
    let file = File::open(archive_path).map_err(|e| AppError::Archive(e.to_string()))?;
    let archive =
        ZipArchive::new(file).map_err(|e| AppError::Archive(format!("zip open: {e}")))?;
    Ok(Arc::new(CachedZip {
        mtime,
        size,
        archive: Mutex::new(archive),
    }))
}

pub fn with_zip_archive<F, T>(archive_path: &str, f: F) -> AppResult<T>
where
    F: FnOnce(&mut ZipArchive<File>) -> AppResult<T>,
{
    let key = canonical_key(archive_path)?;
    let (mtime, size) = fingerprint(archive_path)?;
    let entry = {
        let mut guard = cache()
            .lock()
            .map_err(|e| AppError::Archive(format!("zip cache lock: {e}")))?;
        let stale = guard.get(&key).map(|e| e.mtime != mtime || e.size != size);
        if stale == Some(true) {
            guard.pop(&key);
        }
        if guard.get(&key).is_none() {
            let opened = open_zip(archive_path, mtime, size)?;
            guard.put(key.clone(), opened);
        }
        guard
            .get(&key)
            .cloned()
            .ok_or_else(|| AppError::Archive("zip cache miss".into()))?
    };
    let mut zip = entry
        .archive
        .lock()
        .map_err(|e| AppError::Archive(format!("zip archive lock: {e}")))?;
    f(&mut zip)
}