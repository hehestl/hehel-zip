use super::backend::{BackendKind, ExtractResult, SkippedEntry};
use super::hehe_format::HeheFormat;
use super::path_safety;
use crate::archive::seven_zip::{ArchiveEntryDto, ExtractOptions};
use crate::error::{AppError, AppResult};
use std::path::Path;

pub struct HeheBackend;

impl HeheBackend {
    pub fn probe_magic(magic: &[u8], extension: &str) -> bool {
        magic.starts_with(b"HEHE") || extension.eq_ignore_ascii_case("hehe")
    }

    pub fn probe(path: &str) -> AppResult<bool> {
        HeheFormat::probe(path)
    }
}

impl super::backend::ArchiveBackend for HeheBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Hehe
    }

    fn list(&self, archive_path: &str) -> AppResult<Vec<ArchiveEntryDto>> {
        HeheFormat::list(archive_path)
    }

    fn list_paginated(
        &self,
        archive_path: &str,
        offset: usize,
        limit: usize,
    ) -> AppResult<(Vec<ArchiveEntryDto>, usize)> {
        HeheFormat::list_paginated(archive_path, offset, limit)
    }

    fn extract_entries(
        &self,
        archive_path: &str,
        destination: &str,
        entries: &[String],
        options: &ExtractOptions,
    ) -> AppResult<ExtractResult> {
        HeheFormat::extract(archive_path, destination, entries, options.preserve_paths)
    }

    fn read_entry_bytes(&self, archive_path: &str, entry_path: &str) -> AppResult<Vec<u8>> {
        HeheFormat::read_entry_bytes(archive_path, entry_path)
    }

    fn write_entry_to_path(
        &self,
        archive_path: &str,
        entry_path: &str,
        dest: &Path,
        preserve_paths: bool,
    ) -> AppResult<()> {
        let _ = preserve_paths;
        HeheFormat::extract_entry_to_path(archive_path, entry_path, dest)
    }
}

pub fn safe_out_path(
    destination: &str,
    entry_path: &str,
    preserve_paths: bool,
) -> AppResult<std::path::PathBuf> {
    let dest = Path::new(destination);
    let relative = if preserve_paths {
        entry_path.replace('/', "\\")
    } else {
        Path::new(entry_path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| entry_path.to_string())
    };
    path_safety::resolve_safe_extract_path(dest, &relative)
}

pub fn skip_reason(err: &AppError) -> String {
    err.to_string()
}

pub fn skipped(entry_path: &str, err: AppError) -> SkippedEntry {
    SkippedEntry {
        path: entry_path.to_string(),
        reason: skip_reason(&err),
    }
}
