use crate::archive::archive_service::ArchiveService;
use crate::archive::hehe_format::HeheCreateResult;
use crate::archive::seven_zip::{ArchiveEntryDto, ExtractOptions};
use crate::error::AppResult;
use std::path::Path;

/// Thin compatibility wrapper; prefer `ArchiveService` directly.
pub struct CompositeArchiveAdapter {
    service: ArchiveService,
}

impl CompositeArchiveAdapter {
    pub fn new() -> AppResult<Self> {
        Ok(Self {
            service: ArchiveService::new()?,
        })
    }

    pub fn probe(&self, path: &str) -> AppResult<bool> {
        self.service.probe(path)
    }

    pub fn list(&self, archive_path: &str) -> AppResult<Vec<ArchiveEntryDto>> {
        self.service.list(archive_path)
    }

    pub fn extract(
        &self,
        archive_path: &str,
        destination: &str,
        entries: &[String],
        options: &ExtractOptions,
    ) -> AppResult<Vec<String>> {
        Ok(self
            .service
            .extract(archive_path, destination, entries, options)?
            .written)
    }

    pub fn extract_with_cache(
        &self,
        archive_path: &str,
        session_dir: &str,
        entries: &[String],
        options: &ExtractOptions,
        cache_dir: Option<&str>,
    ) -> AppResult<Vec<String>> {
        self.service
            .extract_with_cache(archive_path, session_dir, entries, options, cache_dir)
    }

    pub fn warm_cache(
        &self,
        archive_path: &str,
        entries: &[String],
        preserve_paths: bool,
        cache_dir: Option<&str>,
    ) -> AppResult<()> {
        self.service
            .warm_cache(archive_path, entries, preserve_paths, cache_dir)
    }

    pub fn create_archive(&self, output_path: &str, file_paths: &[String], preset: Option<&str>, convert_images_to_webp: Option<bool>) -> AppResult<HeheCreateResult> {
        self.service.create_archive(output_path, file_paths, preset, convert_images_to_webp)
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
        self.service
            .create_hehe_from_archive(source_archive, entry_paths, strip_prefix, output_path, preset, convert_images_to_webp)
    }

    pub fn read_entry_bytes(&self, archive_path: &str, entry_path: &str) -> AppResult<Vec<u8>> {
        self.service.read_entry_bytes(archive_path, entry_path)
    }

    pub fn read_hehestl(&self, archive_path: &str) -> AppResult<Option<String>> {
        self.service.read_hehestl(archive_path)
    }

    pub fn service(&self) -> &ArchiveService {
        &self.service
    }
}

pub fn ensure_hehe_extension(output_path: &str) -> String {
    let p = Path::new(output_path);
    if p
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("hehe"))
    {
        return output_path.to_string();
    }
    if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
        if let Some(parent) = p.parent().filter(|parent| !parent.as_os_str().is_empty()) {
            return parent
                .join(format!("{stem}.hehe"))
                .to_string_lossy()
                .into_owned();
        }
        return format!("{stem}.hehe");
    }
    format!("{output_path}.hehe")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_hehe_adds_extension() {
        assert_eq!(ensure_hehe_extension("archive"), "archive.hehe");
        assert_eq!(ensure_hehe_extension("a.zip"), "a.hehe");
        assert_eq!(ensure_hehe_extension("b.hehe"), "b.hehe");
    }
}
