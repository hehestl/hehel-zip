use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;
use std::sync::Arc;
use std::time::{Duration, Instant};

const MAX_BYTES: u64 = 100 * 1024 * 1024;
const TTL: Duration = Duration::from_secs(300);
const MAX_ENTRIES: usize = 256;

struct CacheEntry {
    bytes: Arc<[u8]>,
    mime: String,
    size: u64,
    inserted: Instant,
}

pub struct PreviewCache {
    inner: Mutex<PreviewCacheInner>,
}

struct PreviewCacheInner {
    entries: LruCache<(String, String), CacheEntry>,
    total_bytes: u64,
}

impl PreviewCache {
    pub fn new() -> Self {
        let cap = NonZeroUsize::new(MAX_ENTRIES).unwrap_or(NonZeroUsize::MIN);
        Self {
            inner: Mutex::new(PreviewCacheInner {
                entries: LruCache::new(cap),
                total_bytes: 0,
            }),
        }
    }

    pub fn get(&self, archive_id: &str, entry_path: &str) -> Option<(Arc<[u8]>, String)> {
        let mut inner = self.inner.lock().ok()?;
        let key = (archive_id.to_string(), entry_path.to_string());
        let entry = inner.entries.get(&key)?;
        if entry.inserted.elapsed() > TTL {
            return None;
        }
        Some((Arc::clone(&entry.bytes), entry.mime.clone()))
    }

    pub fn put(&self, archive_id: &str, entry_path: &str, bytes: Vec<u8>, mime: String) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        let key = (archive_id.to_string(), entry_path.to_string());
        let size = bytes.len() as u64;
        let arc_bytes: Arc<[u8]> = Arc::from(bytes.into_boxed_slice());

        if let Some(old) = inner.entries.pop(&key) {
            inner.total_bytes = inner.total_bytes.saturating_sub(old.size);
        }

        while inner.total_bytes + size > MAX_BYTES {
            if let Some((_, evicted)) = inner.entries.pop_lru() {
                inner.total_bytes = inner.total_bytes.saturating_sub(evicted.size);
            } else {
                break;
            }
        }

        inner.total_bytes += size;
        inner.entries.put(
            key,
            CacheEntry {
                bytes: arc_bytes,
                mime,
                size,
                inserted: Instant::now(),
            },
        );
    }
}

pub fn mime_from_path(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        _ => "application/octet-stream",
    }
}
