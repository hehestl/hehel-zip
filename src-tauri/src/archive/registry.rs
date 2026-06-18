use super::backend::use_zip_native;
use super::hehe_backend::HeheBackend;
use super::sevenz_fallback::SevenZFallbackBackend;
use super::zip_backend::ZipBackend;
use crate::archive::seven_zip::SevenZipAdapter;
use crate::error::AppResult;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

pub fn probe_and_create(
    archive_path: &str,
    seven_zip: Arc<SevenZipAdapter>,
) -> AppResult<Arc<dyn super::backend::ArchiveBackend>> {
    let (magic, extension) = read_probe_hints(archive_path)?;

    if HeheBackend::probe_magic(&magic, &extension) || HeheBackend::probe(archive_path)? {
        return Ok(Arc::new(HeheBackend));
    }

    if use_zip_native() && ZipBackend::probe_magic(&magic, &extension) {
        return Ok(Arc::new(ZipBackend::new(seven_zip.clone())));
    }

    if SevenZFallbackBackend::probe_magic(&magic, &extension) {
        return Ok(Arc::new(SevenZFallbackBackend::new(seven_zip)));
    }

    Ok(Arc::new(SevenZFallbackBackend::new(seven_zip)))
}

fn read_probe_hints(path: &str) -> AppResult<(Vec<u8>, String)> {
    let p = Path::new(path);
    let extension = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let mut magic = vec![0u8; 8];
    if let Ok(mut f) = File::open(path) {
        let n = f.read(&mut magic).unwrap_or(0);
        magic.truncate(n);
    }

    Ok((magic, extension))
}

pub fn probe_archive(path: &str, seven_zip: &SevenZipAdapter) -> AppResult<bool> {
    if HeheBackend::probe(path)? {
        return Ok(true);
    }
    seven_zip.probe(path)
}

