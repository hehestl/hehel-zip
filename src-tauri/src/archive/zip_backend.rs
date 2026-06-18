use super::backend::{BackendKind, ExtractResult};
use super::hehe_backend::skipped;
use super::path_safety;
use super::zip_handle_cache;
use crate::archive::seven_zip::{ArchiveEntryDto, ExtractOptions, SevenZipAdapter};
use crate::error::{AppError, AppResult};
use std::fs::{self, File};
use std::io::{copy, Read};
use std::path::Path;
use std::sync::Arc;
use zip::read::ZipFile;

pub struct ZipBackend {
    seven_zip: Arc<SevenZipAdapter>,
}

fn zip_entry_dto(entry: ZipFile<'_>) -> ArchiveEntryDto {
    let name = entry.name().replace('\\', "/");
    let is_dir = entry.is_dir() || name.ends_with('/');
    let size = entry.size();
    let packed = entry.compressed_size();
    let modified = entry.last_modified().map(|dt| {
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            dt.year(),
            dt.month(),
            dt.day(),
            dt.hour(),
            dt.minute(),
            dt.second()
        )
    });
    let extension = Path::new(&name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let base_name = Path::new(&name)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&name)
        .to_string();
    ArchiveEntryDto {
        path: name,
        name: base_name,
        size,
        packed_size: packed,
        modified,
        is_dir,
        extension,
    }
}

impl ZipBackend {
    pub fn new(seven_zip: Arc<SevenZipAdapter>) -> Self {
        Self { seven_zip }
    }

    fn list_range(
        &self,
        archive_path: &str,
        offset: usize,
        limit: usize,
    ) -> AppResult<(usize, Vec<ArchiveEntryDto>)> {
        zip_handle_cache::with_zip_archive(archive_path, |archive| {
            let total = archive.len();
            let end = offset.saturating_add(limit).min(total);
            let mut out = Vec::with_capacity(end.saturating_sub(offset));
            for i in offset..end {
                let entry = archive
                    .by_index(i)
                    .map_err(|e| AppError::Archive(format!("zip entry: {e}")))?;
                out.push(zip_entry_dto(entry));
            }
            Ok((total, out))
        })
    }

    pub fn probe_magic(magic: &[u8], extension: &str) -> bool {
        (magic.starts_with(b"PK\x03\x04") || magic.starts_with(b"PK\x05\x06"))
            && extension.eq_ignore_ascii_case("zip")
    }
}

impl super::backend::ArchiveBackend for ZipBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Zip
    }

    fn list(&self, archive_path: &str) -> AppResult<Vec<ArchiveEntryDto>> {
        let (_, entries) = self.list_range(archive_path, 0, usize::MAX)?;
        Ok(entries)
    }

    fn list_paginated(
        &self,
        archive_path: &str,
        offset: usize,
        limit: usize,
    ) -> AppResult<(Vec<ArchiveEntryDto>, usize)> {
        let (total, entries) = self.list_range(archive_path, offset, limit)?;
        Ok((entries, total))
    }

    fn extract_entries(
        &self,
        archive_path: &str,
        destination: &str,
        entries: &[String],
        options: &ExtractOptions,
    ) -> AppResult<ExtractResult> {
        match try_zip_extract(archive_path, destination, entries, options) {
            Ok(result) => Ok(result),
            Err(reason) => {
                tracing::info!(backend = "sevenz-fallback", reason = %reason, "zip-native extract failed");
                let written = self
                    .seven_zip
                    .extract(archive_path, destination, entries, options)?;
                Ok(ExtractResult::from_written(written))
            }
        }
    }

    fn read_entry_bytes(&self, archive_path: &str, entry_path: &str) -> AppResult<Vec<u8>> {
        let norm = entry_path.replace('\\', "/");
        zip_handle_cache::with_zip_archive(archive_path, |archive| {
            let mut entry = archive
                .by_name(&norm)
                .map_err(|_| AppError::ArchiveEntryNotFound(entry_path.to_string()))?;
            let mut buf = Vec::new();
            entry
                .read_to_end(&mut buf)
                .map_err(|e| AppError::Archive(e.to_string()))?;
            Ok(buf)
        })
    }

    fn write_entry_to_path(
        &self,
        archive_path: &str,
        entry_path: &str,
        dest: &Path,
        preserve_paths: bool,
    ) -> AppResult<()> {
        let _ = preserve_paths;
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| AppError::Archive(e.to_string()))?;
        }
        let norm = entry_path.replace('\\', "/");
        zip_handle_cache::with_zip_archive(archive_path, |archive| {
            let mut entry = archive
                .by_name(&norm)
                .map_err(|_| AppError::ArchiveEntryNotFound(entry_path.to_string()))?;
            let mut out_file = File::create(dest).map_err(|e| AppError::Archive(e.to_string()))?;
            copy(&mut entry, &mut out_file).map_err(|e| AppError::Archive(e.to_string()))?;
            Ok(())
        })
    }
}

fn try_zip_extract(
    archive_path: &str,
    destination: &str,
    entries: &[String],
    options: &ExtractOptions,
) -> Result<ExtractResult, String> {
    let dest = Path::new(destination);
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;

    let targets: Vec<String> = if entries.is_empty() {
        zip_handle_cache::with_zip_archive(archive_path, |archive| {
            Ok((0..archive.len())
                .filter_map(|i| {
                    archive.by_index(i).ok().map(|e| e.name().replace('\\', "/"))
                })
                .collect())
        })
        .map_err(|e| e.to_string())?
    } else {
        entries.to_vec()
    };

    let mut written = Vec::new();
    let mut skipped_list = Vec::new();

    for entry_path in targets {
        let norm = entry_path.replace('\\', "/");
        if norm.ends_with('/') {
            continue;
        }

        let relative = if options.preserve_paths {
            norm.replace('/', "\\")
        } else {
            Path::new(&norm)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| norm.clone())
        };

        let out = match path_safety::resolve_safe_extract_path(dest, &relative) {
            Ok(p) => p,
            Err(e) => {
                skipped_list.push(skipped(&entry_path, e));
                continue;
            }
        };

        let extract_result: Result<(), AppError> =
            zip_handle_cache::with_zip_archive(archive_path, |archive| {
                let mut entry = archive
                    .by_name(&norm)
                    .map_err(|_| AppError::ArchiveEntryNotFound(entry_path.clone()))?;

                if entry.is_dir() {
                    return Ok(());
                }

                if let Some(parent) = out.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| AppError::Archive(e.to_string()))?;
                }

                let mut out_file = File::create(&out)
                    .map_err(|e| AppError::Archive(e.to_string()))?;
                copy(&mut entry, &mut out_file)
                    .map_err(|e| AppError::Archive(e.to_string()))?;
                Ok(())
            });

        match extract_result {
            Ok(()) => written.push(out.to_string_lossy().into_owned()),
            Err(AppError::ArchiveEntryNotFound(_)) => {
                skipped_list.push(skipped(
                    &entry_path,
                    AppError::ArchiveEntryNotFound(entry_path.clone()),
                ));
            }
            Err(e) => skipped_list.push(skipped(&entry_path, e)),
        }
    }

    Ok(ExtractResult {
        written,
        skipped: skipped_list,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archive::backend::ArchiveBackend;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn write_zip(dir: &TempDir) -> PathBuf {
        let zip_path = dir.path().join("test.zip");
        let file = File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("part.stl", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"solid test\nendsolid test\n").unwrap();
        zip.finish().unwrap();
        zip_path
    }

    #[test]
    fn zip_list_paginated_returns_slice() {
        let dir = TempDir::new().unwrap();
        let zip_path = write_zip(&dir);
        let seven = Arc::new(SevenZipAdapter::new().expect("7z"));
        let backend = ZipBackend::new(seven);
        let path = zip_path.to_string_lossy();
        let (page, total) = backend.list_paginated(&path, 0, 1).unwrap();
        assert_eq!(total, 1);
        assert_eq!(page.len(), 1);
        assert_eq!(page[0].name, "part.stl");
    }

    #[test]
    fn zip_backend_lists_and_extracts() {
        let dir = TempDir::new().unwrap();
        let zip_path = write_zip(&dir);
        let seven = Arc::new(SevenZipAdapter::new().expect("7z"));
        let backend = ZipBackend::new(seven);
        let path = zip_path.to_string_lossy();
        let list = backend.list(&path).unwrap();
        assert!(!list.is_empty());

        let out = dir.path().join("out");
        let result = backend
            .extract_entries(
                &path,
                &out.to_string_lossy(),
                &["part.stl".into()],
                &ExtractOptions::default(),
            )
            .unwrap();
        assert!(!result.written.is_empty());
    }
}
