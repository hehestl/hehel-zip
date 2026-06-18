use super::backend::{ArchiveBackend, BackendKind, ExtractResult};
use super::extract_cache;
use super::hehe_backend::HeheBackend;
use super::hehe_format::{HeheCreateOptions, HeheCreateResult, HeheFormat, METADATA_PATH};
use super::registry;
use super::seven_zip::{ArchiveEntryDto, ExtractOptions, SevenZipAdapter};
use crate::archive::adapter::ensure_hehe_extension;
use crate::error::{AppError, AppResult};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use rayon::prelude::*;
use std::sync::{Arc, RwLock};
use std::time::{Instant, SystemTime};
use uuid::Uuid;



#[derive(Clone)]
struct ListingCacheEntry {
    mtime: SystemTime,
    size: u64,
    entries: Vec<ArchiveEntryDto>,
}

pub struct ArchiveService {
    seven_zip: Arc<SevenZipAdapter>,
    open_backends: RwLock<HashMap<String, Arc<dyn ArchiveBackend>>>,
    listing_cache: RwLock<HashMap<String, ListingCacheEntry>>,
}


impl ArchiveService {

    pub fn new() -> AppResult<Self> {

        Ok(Self {
            seven_zip: Arc::new(SevenZipAdapter::new()?),
            open_backends: RwLock::new(HashMap::new()),
            listing_cache: RwLock::new(HashMap::new()),
        })
    }

    fn archive_fingerprint(path: &str) -> AppResult<(SystemTime, u64)> {
        let meta = fs::metadata(path).map_err(|e| AppError::Archive(e.to_string()))?;
        Ok((
            meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            meta.len(),
        ))
    }

    fn get_cached_listing(&self, canonical: &str, mtime: SystemTime, size: u64) -> Option<Vec<ArchiveEntryDto>> {
        let guard = self.listing_cache.read().ok()?;
        let hit = guard.get(canonical)?;
        if hit.mtime == mtime && hit.size == size {
            Some(hit.entries.clone())
        } else {
            None
        }
    }

    fn put_listing_cache(
        &self,
        canonical: &str,
        mtime: SystemTime,
        size: u64,
        entries: Vec<ArchiveEntryDto>,
    ) {
        let Ok(mut guard) = self.listing_cache.write() else {
            return;
        };
        guard.insert(
            canonical.to_string(),
            ListingCacheEntry {
                mtime,
                size,
                entries,
            },
        );
    }


    pub fn open(&self, archive_path: &str) -> AppResult<Arc<dyn ArchiveBackend>> {

        let canonical = super::seven_zip::normalize_archive_path(archive_path)?;

        if let Some(hit) = self

            .open_backends

            .read()

            .map_err(|e| crate::error::AppError::Archive(e.to_string()))?

            .get(&canonical)

            .cloned()

        {

            return Ok(hit);

        }



        let backend = registry::probe_and_create(&canonical, self.seven_zip.clone())?;

        self.open_backends

            .write()

            .map_err(|e| crate::error::AppError::Archive(e.to_string()))?

            .insert(canonical, backend.clone());

        Ok(backend)

    }



    pub fn probe(&self, path: &str) -> AppResult<bool> {

        registry::probe_archive(path, &self.seven_zip)

    }



    pub fn list(&self, archive_path: &str) -> AppResult<Vec<ArchiveEntryDto>> {
        let _span = tracing::info_span!("archive_list", path = %archive_path).entered();
        let start = Instant::now();
        let canonical = super::seven_zip::normalize_archive_path(archive_path)?;
        let (mtime, size) = Self::archive_fingerprint(&canonical)?;

        if let Some(cached) = self.get_cached_listing(&canonical, mtime, size) {
            tracing::info!(
                elapsed_ms = start.elapsed().as_millis(),
                count = cached.len(),
                cache_hit = true,
                "archive_list"
            );
            return Ok(cached);
        }

        let entries = self.open(&canonical)?.list(&canonical)?;
        self.put_listing_cache(&canonical, mtime, size, entries.clone());
        tracing::info!(
            elapsed_ms = start.elapsed().as_millis(),
            count = entries.len(),
            cache_hit = false,
            "archive_list"
        );
        Ok(entries)
    }


    pub fn list_paginated(
        &self,
        archive_path: &str,
        offset: usize,
        limit: usize,
    ) -> AppResult<(Vec<ArchiveEntryDto>, usize)> {
        let _span = tracing::info_span!(
            "archive_list_paginated",
            path = %archive_path,
            offset,
            limit
        )
        .entered();
        let start = Instant::now();
        let canonical = super::seven_zip::normalize_archive_path(archive_path)?;
        let (mtime, size) = Self::archive_fingerprint(&canonical)?;

        if let Some(cached) = self.get_cached_listing(&canonical, mtime, size) {
            let total = cached.len();
            let slice: Vec<ArchiveEntryDto> =
                cached.iter().skip(offset).take(limit).cloned().collect();
            tracing::info!(
                elapsed_ms = start.elapsed().as_millis(),
                count = slice.len(),
                total,
                cache_hit = true,
                "archive_list_paginated"
            );
            return Ok((slice, total));
        }

        let backend = self.open(&canonical)?;
        let (entries, total) = if backend.kind() == BackendKind::SevenZ {
            let all = backend.list(&canonical)?;
            let total = all.len();
            self.put_listing_cache(&canonical, mtime, size, all.clone());
            let slice = all.into_iter().skip(offset).take(limit).collect();
            (slice, total)
        } else {
            let (page, total) = backend.list_paginated(&canonical, offset, limit)?;
            if offset == 0 && page.len() == total {
                self.put_listing_cache(&canonical, mtime, size, page.clone());
            }
            (page, total)
        };

        tracing::info!(
            elapsed_ms = start.elapsed().as_millis(),
            count = entries.len(),
            total,
            cache_hit = false,
            "archive_list_paginated"
        );
        Ok((entries, total))
    }




    pub fn extract(

        &self,

        archive_path: &str,

        destination: &str,

        entries: &[String],

        options: &ExtractOptions,

    ) -> AppResult<ExtractResult> {

        self.open(archive_path)?

            .extract_entries(archive_path, destination, entries, options)

    }



    pub fn extract_with_cache(

        &self,

        archive_path: &str,

        session_dir: &str,

        entries: &[String],

        options: &ExtractOptions,

        cache_dir: Option<&str>,

    ) -> AppResult<Vec<String>> {

        extract_cache::set_custom_root(cache_dir.map(str::to_string));



        if entries.is_empty() {

            return Ok(self

                .extract(archive_path, session_dir, entries, options)?

                .written);

        }



        let archive_hash = extract_cache::archive_hash(archive_path)?;

        self.populate_cache_entries(

            archive_path,

            entries,

            options.preserve_paths,

            &archive_hash,

        )?;



        let session = PathBuf::from(session_dir);

        fs::create_dir_all(&session).map_err(|e| crate::error::AppError::Archive(e.to_string()))?;

        let mut written = Vec::with_capacity(entries.len());



        for entry_path in entries {

            let cache_path = extract_cache::get_if_exists_with_hash(

                &archive_hash,

                entry_path,

                options.preserve_paths,

            )

            .ok_or_else(|| {

                crate::error::AppError::Archive(format!(

                    "cache miss after populate: {entry_path}"

                ))

            })?;

            let out =

                extract_cache::session_out_path(&session, entry_path, options.preserve_paths);

            extract_cache::link_or_copy(&cache_path, &out)?;

            written.push(out.to_string_lossy().into_owned());

        }

        Ok(written)

    }



    pub fn warm_cache(

        &self,

        archive_path: &str,

        entries: &[String],

        preserve_paths: bool,

        cache_dir: Option<&str>,

    ) -> AppResult<()> {

        extract_cache::set_custom_root(cache_dir.map(str::to_string));

        if entries.is_empty() {

            return Ok(());

        }

        let archive_hash = extract_cache::archive_hash(archive_path)?;

        self.populate_cache_entries(archive_path, entries, preserve_paths, &archive_hash)

    }



    fn populate_cache_entries(

        &self,

        archive_path: &str,

        entries: &[String],

        preserve_paths: bool,

        archive_hash: &str,

    ) -> AppResult<()> {

        let mut misses = Vec::new();

        for entry_path in entries {

            if extract_cache::get_if_exists_with_hash(archive_hash, entry_path, preserve_paths)

                .is_none()

            {

                misses.push(entry_path.clone());

            }

        }

        if misses.is_empty() {

            return Ok(());

        }



        let backend = self.open(archive_path)?;

        let start = Instant::now();

        let kind = backend.kind();



        if kind == BackendKind::SevenZ && misses.len() > 1 {

            self.seven_zip_batch_to_cache(

                archive_path,

                &misses,

                preserve_paths,

                archive_hash,

            )?;

        } else if misses.len() == 1 {
            self.write_cache_miss(
                archive_path,
                &misses[0],
                preserve_paths,
                archive_hash,
                backend.as_ref(),
            )?;
        } else {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(4)
                .build()
                .map_err(|e| AppError::Archive(e.to_string()))?;
            let path = archive_path.to_string();
            let hash = archive_hash.to_string();
            let preserve = preserve_paths;
            pool.install(|| -> AppResult<()> {
                misses.par_iter().try_for_each(|entry_path| -> AppResult<()> {
                    self.write_cache_miss(
                        &path,
                        entry_path,
                        preserve,
                        &hash,
                        backend.as_ref(),
                    )?;
                    Ok(())
                })?;
                Ok(())
            })?;
        }


        tracing::info!(

            backend = ?kind,

            count = misses.len(),

            elapsed_ms = start.elapsed().as_millis(),

            cache_hit = false,

            "cache_populate"

        );

        Ok(())

    }



    fn write_cache_miss(

        &self,

        archive_path: &str,

        entry_path: &str,

        preserve_paths: bool,

        archive_hash: &str,

        backend: &dyn ArchiveBackend,

    ) -> AppResult<PathBuf> {

        let start = Instant::now();

        let (part, dest) =

            extract_cache::prepare_cache_write(archive_hash, entry_path, preserve_paths)?;

        backend.write_entry_to_path(archive_path, entry_path, &part, preserve_paths)?;

        let cached = extract_cache::finalize_part(&part, &dest)?;

        tracing::info!(

            backend = ?backend.kind(),

            entry = %entry_path,

            elapsed_ms = start.elapsed().as_millis(),

            cache_hit = false,

            "ensure_cached_entry"

        );

        Ok(cached)

    }



    fn seven_zip_batch_to_cache(

        &self,

        archive_path: &str,

        misses: &[String],

        preserve_paths: bool,

        archive_hash: &str,

    ) -> AppResult<()> {

        let staging = std::env::temp_dir().join(format!("hehel-7z-batch-{}", Uuid::new_v4()));

        let result = (|| {

            self.seven_zip.extract_entries_to_dir(

                archive_path,

                misses,

                &staging,

                preserve_paths,

            )?;

            for entry_path in misses {

                let extracted = super::seven_zip::staging_entry_path(

                    &staging,

                    entry_path,

                    preserve_paths,

                )?;

                let (part, dest) =

                    extract_cache::prepare_cache_write(archive_hash, entry_path, preserve_paths)?;

                if part.exists() {

                    fs::remove_file(&part).ok();

                }

                fs::rename(&extracted, &part).map_err(|e| {

                    crate::error::AppError::Archive(format!("7z batch stage: {e}"))

                })?;

                extract_cache::finalize_part(&part, &dest)?;

            }

            Ok(())

        })();

        let _ = fs::remove_dir_all(&staging);

        result

    }



    pub fn create_archive(
        &self,
        output_path: &str,
        file_paths: &[String],
        preset: Option<&str>,
        convert_images_to_webp: Option<bool>,
    ) -> AppResult<HeheCreateResult> {
        let output = ensure_hehe_extension(output_path);
        let options = HeheCreateOptions::from_api(preset, convert_images_to_webp);
        HeheFormat::create(&output, file_paths, options)
    }

    pub fn create_hehe_from_archive(
        &self,
        source_archive: &str,
        entry_paths: &[String],
        strip_prefix: Option<&str>,
        output_path: &str,
        preset: Option<&str>,
        convert_images_to_webp: Option<bool>,
    ) -> AppResult<HeheCreateResult> {
        let output = ensure_hehe_extension(output_path);
        let options = HeheCreateOptions::from_api(preset, convert_images_to_webp);
        HeheFormat::create_from_archive(source_archive, entry_paths, strip_prefix, &output, options)
    }

    pub fn read_entry_bytes(&self, archive_path: &str, entry_path: &str) -> AppResult<Vec<u8>> {

        self.open(archive_path)?

            .read_entry_bytes(archive_path, entry_path)

    }



    pub fn read_hehestl(&self, archive_path: &str) -> AppResult<Option<String>> {
        let _span = tracing::info_span!("read_hehestl", path = %archive_path).entered();
        let start = Instant::now();

        if HeheBackend::probe(archive_path)? {
            let result = HeheFormat::read_metadata(archive_path)?;
            tracing::info!(
                elapsed_ms = start.elapsed().as_millis(),
                found = result.is_some(),
                "read_hehestl"
            );
            return Ok(result);
        }

        let result = self.try_read_hehestl_at(archive_path, METADATA_PATH)?;
        tracing::info!(
            elapsed_ms = start.elapsed().as_millis(),
            found = result.is_some(),
            "read_hehestl"
        );
        Ok(result)
    }

    fn try_read_hehestl_at(&self, archive_path: &str, entry_path: &str) -> AppResult<Option<String>> {
        match self.read_entry_bytes(archive_path, entry_path) {
            Ok(bytes) => Ok(Some(String::from_utf8_lossy(&bytes).into_owned())),
            Err(AppError::ArchiveEntryNotFound(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    #[cfg(test)]
    pub fn insert_backend_for_test(&self, canonical: &str, backend: Arc<dyn ArchiveBackend>) {
        if let Ok(mut guard) = self.open_backends.write() {
            guard.insert(canonical.to_string(), backend);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archive::backend::ArchiveBackend;
    use crate::archive::seven_zip;
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::TempDir;

    struct CountingBackend {
        list_calls: AtomicUsize,
        entries: Vec<ArchiveEntryDto>,
    }

    impl ArchiveBackend for CountingBackend {
        fn kind(&self) -> BackendKind {
            BackendKind::Zip
        }

        fn list(&self, _archive_path: &str) -> AppResult<Vec<ArchiveEntryDto>> {
            self.list_calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.entries.clone())
        }

        fn extract_entries(
            &self,
            _archive_path: &str,
            _destination: &str,
            _entries: &[String],
            _options: &ExtractOptions,
        ) -> AppResult<ExtractResult> {
            Ok(ExtractResult::from_written(Vec::new()))
        }

        fn read_entry_bytes(&self, _archive_path: &str, entry_path: &str) -> AppResult<Vec<u8>> {
            if entry_path == METADATA_PATH {
                Ok(b"ArchiveId: 550e8400-e29b-41d4-a716-446655440000\n".to_vec())
            } else {
                Err(AppError::ArchiveEntryNotFound(entry_path.to_string()))
            }
        }
    }

    fn write_zip_with_metadata(dir: &TempDir) -> PathBuf {
        use std::fs::File;
        let zip_path = dir.path().join("meta.zip");
        let file = File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file(METADATA_PATH, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"ArchiveId: 550e8400-e29b-41d4-a716-446655440000\n")
            .unwrap();
        zip.start_file("part.stl", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"solid test\n").unwrap();
        zip.finish().unwrap();
        zip_path
    }

    #[test]
    fn read_hehestl_reads_metadata_without_listing() {
        let dir = TempDir::new().unwrap();
        let zip_path = write_zip_with_metadata(&dir);
        let service = ArchiveService::new().expect("service");
        let path = seven_zip::normalize_archive_path(&zip_path.to_string_lossy()).unwrap();
        let backend = Arc::new(CountingBackend {
            list_calls: AtomicUsize::new(0),
            entries: vec![ArchiveEntryDto {
                path: "part.stl".into(),
                name: "part.stl".into(),
                size: 10,
                packed_size: 5,
                modified: None,
                is_dir: false,
                extension: "stl".into(),
            }],
        });
        service.insert_backend_for_test(&path, backend.clone());
        let meta = service.read_hehestl(&path).unwrap();
        assert!(meta.is_some());
        assert!(meta.unwrap().contains("ArchiveId:"));
        assert_eq!(backend.list_calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn listing_cache_avoids_second_backend_list() {
        let dir = TempDir::new().unwrap();
        let zip_path = write_zip_with_metadata(&dir);
        let path = seven_zip::normalize_archive_path(&zip_path.to_string_lossy()).unwrap();
        let backend = Arc::new(CountingBackend {
            list_calls: AtomicUsize::new(0),
            entries: vec![ArchiveEntryDto {
                path: "part.stl".into(),
                name: "part.stl".into(),
                size: 10,
                packed_size: 5,
                modified: None,
                is_dir: false,
                extension: "stl".into(),
            }],
        });
        let service = ArchiveService::new().expect("service");
        service.insert_backend_for_test(&path, backend.clone());

        let first = service.list(&path).unwrap();
        let second = service.list(&path).unwrap();
        assert_eq!(first.len(), second.len());
        assert_eq!(backend.list_calls.load(Ordering::SeqCst), 1);
    }
}
