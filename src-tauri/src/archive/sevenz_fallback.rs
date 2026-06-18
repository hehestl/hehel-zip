use super::backend::{BackendKind, ExtractResult};
use super::path_safety;
use crate::archive::seven_zip::{ArchiveEntryDto, ExtractOptions, SevenZipAdapter};
use crate::error::AppResult;
use std::path::Path;
use std::sync::Arc;

pub struct SevenZFallbackBackend {
    seven_zip: Arc<SevenZipAdapter>,
}

impl SevenZFallbackBackend {
    pub fn new(seven_zip: Arc<SevenZipAdapter>) -> Self {
        Self { seven_zip }
    }

    pub fn probe_magic(magic: &[u8], extension: &str) -> bool {
        if magic.starts_with(b"PK\x03\x04") || magic.starts_with(b"PK\x05\x06") {
            return true;
        }
        if magic.starts_with(&[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C]) {
            return true;
        }
        if magic.len() >= 7 && magic[0..6] == *b"Rar!\x1A\x07" {
            return true;
        }
        matches!(
            extension.to_ascii_lowercase().as_str(),
            "zip" | "rar" | "7z" | "tar" | "gz"
        )
    }
}

impl super::backend::ArchiveBackend for SevenZFallbackBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::SevenZ
    }

    fn list(&self, archive_path: &str) -> AppResult<Vec<ArchiveEntryDto>> {
        self.seven_zip.list(archive_path)
    }

    fn list_paginated(
        &self,
        archive_path: &str,
        offset: usize,
        limit: usize,
    ) -> AppResult<(Vec<ArchiveEntryDto>, usize)> {
        self.seven_zip
            .list_paginated(archive_path, offset, limit)
    }

    fn extract_entries(
        &self,
        archive_path: &str,
        destination: &str,
        entries: &[String],
        options: &ExtractOptions,
    ) -> AppResult<ExtractResult> {
        if entries.len() == 1 {
            return extract_single_safe(self, archive_path, destination, &entries[0], options);
        }
        let written = self
            .seven_zip
            .extract(archive_path, destination, entries, options)?;
        Ok(ExtractResult::from_written(written))
    }

    fn read_entry_bytes(&self, archive_path: &str, entry_path: &str) -> AppResult<Vec<u8>> {
        self.seven_zip
            .extract_entry_stdout(archive_path, entry_path)
    }

    fn write_entry_to_path(
        &self,
        archive_path: &str,
        entry_path: &str,
        dest: &Path,
        preserve_paths: bool,
    ) -> AppResult<()> {
        self.seven_zip
            .extract_entry_to_path(archive_path, entry_path, dest, preserve_paths)
    }
}

fn extract_single_safe(
    backend: &SevenZFallbackBackend,
    archive_path: &str,
    destination: &str,
    entry: &str,
    options: &ExtractOptions,
) -> AppResult<ExtractResult> {
    let dest = Path::new(destination);
    std::fs::create_dir_all(dest)?;

    let relative = if options.preserve_paths {
        entry.replace('/', "\\")
    } else {
        Path::new(entry)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| entry.to_string())
    };

    match path_safety::resolve_safe_extract_path(dest, &relative) {
        Ok(out) => {
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent)?;
            }
            backend.seven_zip.extract_entry_to_path(
                archive_path,
                entry,
                &out,
                options.preserve_paths,
            )?;
            Ok(ExtractResult::from_written(vec![out.to_string_lossy().into_owned()]))
        }
        Err(e) => Ok(ExtractResult {
            written: Vec::new(),
            skipped: vec![super::hehe_backend::skipped(entry, e)],
        }),
    }
}
