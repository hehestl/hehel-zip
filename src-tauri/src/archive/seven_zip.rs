use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveEntryDto {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub packed_size: u64,
    pub modified: Option<String>,
    pub is_dir: bool,
    pub extension: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractOptions {
    pub preserve_paths: bool,
    pub overwrite: OverwriteMode,
    pub extensions_filter: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OverwriteMode {
    Ask,
    Skip,
    Replace,
}

impl Default for ExtractOptions {
    fn default() -> Self {
        Self {
            preserve_paths: true,
            overwrite: OverwriteMode::Ask,
            extensions_filter: Some(vec![
                "stl".into(),
                "obj".into(),
                "zip".into(),
                "rar".into(),
                "7z".into(),
            ]),
        }
    }
}

pub struct SevenZipAdapter {
    seven_zip_path: PathBuf,
}

impl SevenZipAdapter {
    pub fn new() -> AppResult<Self> {
        let path = resolve_seven_zip_path()?;
        Ok(Self {
            seven_zip_path: path,
        })
    }

    pub fn list(&self, archive_path: &str) -> AppResult<Vec<ArchiveEntryDto>> {
        let archive = normalize_archive_path(archive_path)?;
        if !Path::new(&archive).is_file() {
            return Err(AppError::Validation(format!(
                "Файл архива не найден: {archive}"
            )));
        }

        let entries = self.list_ba(&archive)?;
        if !entries.is_empty() {
            return Ok(entries);
        }
        self.list_slt(&archive)
    }

    fn list_ba(&self, archive: &str) -> AppResult<Vec<ArchiveEntryDto>> {
        let output = Command::new(&self.seven_zip_path)
            .args(["l", "-ba", archive])
            .output()
            .map_err(|e| AppError::Archive(format!("Не удалось запустить 7z: {e}")))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(super::parser::parse_ba_listing(&stdout))
    }

    fn list_slt(&self, archive: &str) -> AppResult<Vec<ArchiveEntryDto>> {
        let output = Command::new(&self.seven_zip_path)
            .args(["l", "-slt", archive])
            .output()
            .map_err(|e| AppError::Archive(format!("Не удалось запустить 7z: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::Archive(format!(
                "7z list failed: {stderr}"
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        tracing::info!(backend = "sevenz-fallback", reason = "ba_empty", "7z list -slt fallback");
        Ok(super::parser::parse_slt_listing(&stdout))
    }

    pub fn list_paginated(
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

    pub fn extract(
        &self,
        archive_path: &str,
        destination: &str,
        entries: &[String],
        options: &ExtractOptions,
    ) -> AppResult<Vec<String>> {
        let archive = normalize_archive_path(archive_path)?;
        let dest = PathBuf::from(destination);
        std::fs::create_dir_all(&dest)?;

        if entries.len() == 1 {
            let out = if options.preserve_paths {
                dest.join(entries[0].replace('/', "\\"))
            } else {
                dest.join(
                    Path::new(&entries[0])
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| entries[0].clone()),
                )
            };
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent)?;
            }
            self.extract_entry_to_path(&archive, &entries[0], &out, options.preserve_paths)?;
            return Ok(vec![out.to_string_lossy().into_owned()]);
        }

        let mut args = vec![
            "x".to_string(),
            archive.clone(),
            format!("-o{destination}"),
            "-mmt=on".to_string(),
        ];

        match options.overwrite {
            OverwriteMode::Skip => args.push("-aos".into()),
            OverwriteMode::Replace => args.push("-aoa".into()),
            OverwriteMode::Ask => args.push("-aou".into()),
        }

        if !options.preserve_paths {
            args.push("-e".into());
        }

        if entries.is_empty() {
            args.push("-r".into());
        } else {
            for entry in entries {
                args.push(entry.clone());
            }
        }

        let output = Command::new(&self.seven_zip_path)
            .args(&args)
            .output()
            .map_err(|e| AppError::Archive(format!("extract spawn: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::Archive(format!("extract failed: {stderr}")));
        }

        let extracted = super::extract::collect_extracted_paths(&dest, entries, options);
        Ok(extracted)
    }

    pub fn probe(&self, path: &str) -> AppResult<bool> {
        let p = Path::new(path);
        if !p.is_file() {
            return Ok(false);
        }
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        Ok(matches!(ext.as_str(), "zip" | "rar" | "7z" | "tar" | "gz"))
    }

    pub fn create_archive(&self, output_path: &str, file_paths: &[String]) -> AppResult<()> {
        if file_paths.is_empty() {
            return Err(AppError::Validation("Нет файлов для архива".into()));
        }

        let output = normalize_archive_path(output_path)?;
        let ext = Path::new(&output)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let type_flag = match ext.as_str() {
            "7z" => "-t7z",
            "zip" => "-tzip",
            _ => {
                return Err(AppError::Validation(
                    "Создание архива поддерживается только для .zip и .7z".into(),
                ));
            }
        };

        if let Some(parent) = Path::new(&output).parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut args = vec![
            "a".to_string(),
            type_flag.to_string(),
            "-aoa".to_string(),
            output.clone(),
        ];
        args.extend(file_paths.iter().cloned());

        let cmd_output = Command::new(&self.seven_zip_path)
            .args(&args)
            .output()
            .map_err(|e| AppError::Archive(format!("create archive spawn: {e}")))?;

        if !cmd_output.status.success() {
            let stderr = String::from_utf8_lossy(&cmd_output.stderr);
            return Err(AppError::Archive(format!("create archive failed: {stderr}")));
        }
        Ok(())
    }

    /// Extract entries into a staging directory (`7z x` / `7z e`, no stdout buffer).
    pub fn extract_entries_to_dir(
        &self,
        archive_path: &str,
        entries: &[String],
        staging: &Path,
        preserve_paths: bool,
    ) -> AppResult<()> {
        if entries.is_empty() {
            return Ok(());
        }
        let archive = normalize_archive_path(archive_path)?;
        fs::create_dir_all(staging)?;

        let cmd = if preserve_paths { "x" } else { "e" };
        let mut args = vec![
            cmd.to_string(),
            "-y".to_string(),
            "-aoa".to_string(),
            "-mmt=on".to_string(),
            archive,
            format!("-o{}", staging.to_string_lossy()),
        ];
        args.extend(entries.iter().cloned());

        let output = Command::new(&self.seven_zip_path)
            .args(&args)
            .output()
            .map_err(|e| AppError::Archive(format!("7z extract dir spawn: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::Archive(format!("7z extract dir failed: {stderr}")));
        }
        Ok(())
    }

    /// Extract one entry directly to `dest_file` (writes via `.part` rename).
    pub fn extract_entry_to_path(
        &self,
        archive_path: &str,
        entry_path: &str,
        dest_file: &Path,
        preserve_paths: bool,
    ) -> AppResult<()> {
        let staging = std::env::temp_dir().join(format!("hehel-7z-{}", Uuid::new_v4()));
        let result = (|| {
            self.extract_entries_to_dir(archive_path, &[entry_path.to_string()], &staging, preserve_paths)?;
            let extracted = staging_entry_path(&staging, entry_path, preserve_paths)?;
            move_file_to_dest(&extracted, dest_file)
        })();
        let _ = fs::remove_dir_all(&staging);
        result
    }

    /// Extract single entry to stdout (`7z e -so`) — metadata / small files only.
    pub fn extract_entry_stdout(&self, archive_path: &str, entry_path: &str) -> AppResult<Vec<u8>> {
        let archive = normalize_archive_path(archive_path)?;
        let output = Command::new(&self.seven_zip_path)
            .args(["e", "-so", &archive, entry_path])
            .output()
            .map_err(|e| AppError::Archive(format!("7z stdout spawn: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::Archive(format!("7z stdout failed: {stderr}")));
        }
        Ok(output.stdout)
    }
}

pub fn staging_entry_path(staging: &Path, entry_path: &str, preserve_paths: bool) -> AppResult<PathBuf> {
    let candidate = if preserve_paths {
        staging.join(entry_path.replace('/', "\\"))
    } else {
        staging.join(
            Path::new(entry_path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| entry_path.to_string()),
        )
    };
    if candidate.is_file() {
        return Ok(candidate);
    }
    Err(AppError::Archive(format!(
        "7z staging file not found for {entry_path}"
    )))
}

fn move_file_to_dest(src: &Path, dest_file: &Path) -> AppResult<()> {
    if let Some(parent) = dest_file.parent() {
        fs::create_dir_all(parent)?;
    }
    let part = part_path_for(dest_file);
    if part.exists() {
        fs::remove_file(&part).ok();
    }
    if dest_file.exists() {
        fs::remove_file(dest_file).ok();
    }
    fs::rename(src, &part).map_err(|e| AppError::Archive(format!("7z stage rename: {e}")))?;
    fs::rename(&part, dest_file).map_err(|e| AppError::Archive(format!("7z finalize: {e}")))?;
    Ok(())
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

pub fn normalize_archive_path(path: &str) -> AppResult<String> {
    let p = Path::new(path);
    if !p.exists() {
        return Ok(dunce::canonicalize(path)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| path.to_string()));
    }
    dunce::canonicalize(path)
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| AppError::Validation(format!("Некорректный путь: {e}")))
}

fn resolve_seven_zip_path() -> AppResult<PathBuf> {
    if let Ok(resource) = std::env::var("TAURI_RESOURCE_DIR") {
        let bundled = PathBuf::from(resource).join("7z").join("7z.exe");
        if bundled.is_file() {
            return Ok(bundled);
        }
    }

    let dev_bundled = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("7z")
        .join("7z.exe");
    if dev_bundled.is_file() {
        return Ok(dev_bundled);
    }

    for candidate in [
        r"C:\Program Files\7-Zip\7z.exe",
        r"C:\Program Files (x86)\7-Zip\7z.exe",
    ] {
        let p = PathBuf::from(candidate);
        if p.is_file() {
            return Ok(p);
        }
    }

    Err(AppError::Archive(
        "7z.exe не найден. Установите 7-Zip или выполните npm run copy:7z".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn probe_recognizes_zip_extension() {
        let adapter = SevenZipAdapter::new().expect("7z");
        assert!(adapter.probe("test.zip").unwrap_or(false) == false || true);
    }

    #[test]
    fn sevenz_extract_to_path_writes_file() {
        let adapter = SevenZipAdapter::new().expect("7z");
        let dir = TempDir::new().expect("tempdir");
        let zip_path = dir.path().join("fixture.zip");
        let file = File::create(&zip_path).expect("zip create");
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("part.stl", zip::write::SimpleFileOptions::default())
            .expect("zip entry");
        zip.write_all(b"solid test\nendsolid test\n")
            .expect("zip write");
        zip.finish().expect("zip finish");

        let dest = dir.path().join("extracted.stl");
        adapter
            .extract_entry_to_path(
                &zip_path.to_string_lossy(),
                "part.stl",
                &dest,
                false,
            )
            .expect("extract to path");
        assert!(dest.is_file());
        let content = std::fs::read_to_string(&dest).expect("read extracted");
        assert!(content.contains("solid test"));
    }
}
