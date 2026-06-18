use crate::archive::seven_zip::{ArchiveEntryDto, ExtractOptions};
use crate::error::AppResult;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkippedEntry {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractResult {
    pub written: Vec<String>,
    pub skipped: Vec<SkippedEntry>,
}

impl ExtractResult {
    pub fn from_written(written: Vec<String>) -> Self {
        Self {
            written,
            skipped: Vec::new(),
        }
    }

    pub fn merge(mut self, other: ExtractResult) -> Self {
        self.written.extend(other.written);
        self.skipped.extend(other.skipped);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Hehe,
    Zip,
    SevenZ,
}

pub trait ArchiveBackend: Send + Sync {
    fn kind(&self) -> BackendKind;

    fn list(&self, archive_path: &str) -> AppResult<Vec<ArchiveEntryDto>>;

    fn list_paginated(
        &self,
        archive_path: &str,
        offset: usize,
        limit: usize,
    ) -> AppResult<(Vec<ArchiveEntryDto>, usize)> {
        let all = self.list(archive_path)?;
        let total = all.len();
        let slice = all.into_iter().skip(offset).take(limit).collect();
        Ok((slice, total))
    }

    fn extract_entries(
        &self,
        archive_path: &str,
        destination: &str,
        entries: &[String],
        options: &ExtractOptions,
    ) -> AppResult<ExtractResult>;

    fn read_entry_bytes(&self, archive_path: &str, entry_path: &str) -> AppResult<Vec<u8>>;

    fn write_entry_to_path(
        &self,
        archive_path: &str,
        entry_path: &str,
        dest: &Path,
        preserve_paths: bool,
    ) -> AppResult<()> {
        let bytes = self.read_entry_bytes(archive_path, entry_path)?;
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(dest, &bytes).map_err(|e| {
            crate::error::AppError::Archive(format!("write entry {entry_path}: {e}"))
        })?;
        Ok(())
    }
}

pub fn use_zip_native() -> bool {
    if !cfg!(feature = "zip-native") {
        return false;
    }
    !matches!(
        std::env::var("HEHEL_USE_ZIP_NATIVE").as_deref(),
        Ok("0") | Ok("false") | Ok("no")
    )
}
