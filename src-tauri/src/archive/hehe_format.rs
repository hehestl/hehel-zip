use crate::archive::backend::{ExtractResult, SkippedEntry};
use crate::archive::image_webp::apply_webp_conversion;
use crate::archive::path_safety;
use crate::archive::seven_zip::ArchiveEntryDto;
use crate::error::{AppError, AppResult};
use chrono::Utc;
use crc32fast::Hasher as Crc32;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};
use std::thread;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

pub const MAGIC: &[u8; 4] = b"HEHE";
pub const VERSION: u16 = 1;
pub const HEADER_SIZE: u64 = 32;
pub const METADATA_PATH: &str = "metadata.hehestl";
/// Default zstd level for balanced preset.
pub const ZSTD_LEVEL_CREATE: i32 = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeheCompression {
    pub zstd_level: i32,
    pub window_log: Option<u32>,
    pub compress_stl: bool,
}

impl HeheCompression {
    pub fn balanced() -> Self {
        Self {
            zstd_level: ZSTD_LEVEL_CREATE,
            window_log: None,
            compress_stl: true,
        }
    }

    pub fn parse_preset(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "fast" => Self {
                zstd_level: 3,
                window_log: None,
                compress_stl: false,
            },
            "ultra" => Self {
                zstd_level: 19,
                window_log: Some(27),
                compress_stl: true,
            },
            _ => Self::balanced(),
        }
    }

    pub fn metadata_label(&self) -> String {
        match self.window_log {
            Some(w) => format!("zstd:{}:w{}", self.zstd_level, w),
            None => format!("zstd:{}", self.zstd_level),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct HeheCreateOptions {
    pub compression: HeheCompression,
    pub convert_images_to_webp: bool,
}

impl HeheCreateOptions {
    pub fn from_api(preset: Option<&str>, convert_images_to_webp: Option<bool>) -> Self {
        Self {
            compression: preset
                .map(HeheCompression::parse_preset)
                .unwrap_or_else(HeheCompression::balanced),
            convert_images_to_webp: convert_images_to_webp.unwrap_or(false),
        }
    }
}

pub const METHOD_STORE: u8 = 0;
pub const METHOD_DEFLATE: u8 = 1;
pub const METHOD_ZSTD: u8 = 2;

const SKIP_NAMES: &[&str] = &["Thumbs.db", ".DS_Store", "desktop.ini", ".gitignore"];
const SKIP_DIR_NAMES: &[&str] = &[".git", "node_modules", "__MACOSX"];
const SYSTEM_METADATA_KEYS: &[&str] = &[
    "FormatVersion",
    "ArchiveId",
    "archive_id",
    "Created",
    "Compression",
    "OriginalSources",
];
const RENAME_MAX_ATTEMPTS: u32 = 3;
const RENAME_RETRY_DELAY_MS: u64 = 75;

#[derive(Debug, Clone)]
struct TocEntry {
    path: String,
    method: u8,
    crc32: u32,
    comp_size: u64,
    raw_size: u64,
    data_offset: u64,
}

#[derive(Debug, Clone)]
pub struct HeheCreateResult {
    pub archive_id: String,
    pub entry_count: u32,
    pub total_bytes: u64,
}

#[derive(Clone)]
struct CachedToc {
    mtime: SystemTime,
    size: u64,
    entries: Vec<TocEntry>,
    by_path: HashMap<String, TocEntry>,
}

static TOC_CACHE: OnceLock<RwLock<HashMap<String, CachedToc>>> = OnceLock::new();

pub struct HeheFormat;

impl HeheFormat {
    pub fn probe(path: &str) -> AppResult<bool> {
        let mut f = File::open(path).map_err(|e| AppError::Archive(e.to_string()))?;
        let mut magic = [0u8; 4];
        if f.read_exact(&mut magic).is_err() {
            return Ok(false);
        }
        Ok(&magic == MAGIC)
    }

    pub fn list(path: &str) -> AppResult<Vec<ArchiveEntryDto>> {
        let (entries, _) = Self::list_paginated(path, 0, usize::MAX)?;
        Ok(entries)
    }

    pub fn list_paginated(
        path: &str,
        offset: usize,
        limit: usize,
    ) -> AppResult<(Vec<ArchiveEntryDto>, usize)> {
        let toc = Self::read_toc(path)?;
        let total = toc.len();
        let end = offset.saturating_add(limit).min(total);
        let slice = toc[offset..end]
            .iter()
            .map(|e| entry_dto(&e.path, e.raw_size, e.comp_size))
            .collect();
        Ok((slice, total))
    }

    pub fn read_entry_bytes(path: &str, entry_path: &str) -> AppResult<Vec<u8>> {
        let norm = normalize_entry_path(entry_path);
        let cached = Self::load_toc_cached(path)?;
        let entry = cached
            .by_path
            .get(&norm)
            .cloned()
            .ok_or_else(|| AppError::Archive(format!("entry not found: {entry_path}")))?;
        Self::read_entry_raw(path, &entry)
    }

    pub fn extract(
        path: &str,
        destination: &str,
        entries: &[String],
        preserve_paths: bool,
    ) -> AppResult<ExtractResult> {
        let cached = Self::load_toc_cached(path)?;
        std::fs::create_dir_all(destination)?;
        let mut archive =
            File::open(path).map_err(|e| AppError::Archive(e.to_string()))?;

        if entries.is_empty() {
            return Self::extract_entries(
                &mut archive,
                &cached,
                destination,
                preserve_paths,
                cached.entries.iter().collect(),
            );
        }

        let filter_set: HashSet<String> = entries
            .iter()
            .map(|e| normalize_entry_path(e))
            .collect();
        let targets = collect_target_entries(&cached, &filter_set);
        Self::extract_entries(&mut archive, &cached, destination, preserve_paths, targets)
    }

    fn extract_entries<'a>(
        archive: &mut File,
        _cached: &CachedToc,
        destination: &str,
        preserve_paths: bool,
        targets: Vec<&'a TocEntry>,
    ) -> AppResult<ExtractResult> {
        let dest = Path::new(destination);
        let mut written = Vec::with_capacity(targets.len());
        let mut skipped = Vec::new();

        for entry in targets {
            if entry.path.ends_with('/') {
                continue;
            }
            let relative = if preserve_paths {
                entry.path.replace('/', "\\")
            } else {
                Path::new(&entry.path)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| entry.path.clone())
            };

            let out = match path_safety::resolve_safe_extract_path(dest, &relative) {
                Ok(p) => p,
                Err(e) => {
                    skipped.push(SkippedEntry {
                        path: entry.path.clone(),
                        reason: e.to_string(),
                    });
                    continue;
                }
            };

            if let Err(e) = extract_entry_to_file(archive, entry, &out) {
                skipped.push(SkippedEntry {
                    path: entry.path.clone(),
                    reason: e.to_string(),
                });
                continue;
            }
            written.push(out.to_string_lossy().into_owned());
        }
        Ok(ExtractResult { written, skipped })
    }

    /// Single entry extract to an explicit destination path (cache/session).
    pub fn extract_entry_to_path(
        archive_path: &str,
        entry_path: &str,
        dest: &Path,
    ) -> AppResult<()> {
        let cached = Self::load_toc_cached(archive_path)?;
        let norm = normalize_entry_path(entry_path);
        let entry = cached
            .by_path
            .get(&norm)
            .ok_or_else(|| AppError::ArchiveEntryNotFound(entry_path.to_string()))?;
        let mut archive =
            File::open(archive_path).map_err(|e| AppError::Archive(e.to_string()))?;
        extract_entry_to_file(&mut archive, entry, dest)
    }

    pub fn create(output_path: &str, file_paths: &[String], options: HeheCreateOptions) -> AppResult<HeheCreateResult> {
        if file_paths.is_empty() {
            return Err(AppError::Validation("Нет файлов для архива".into()));
        }

        let archive_id = Uuid::new_v4().to_string();
        let (mut files, root_metadata) = collect_local_sources(file_paths)?;
        if options.convert_images_to_webp {
            apply_webp_conversion(&mut files)?;
        }
        let metadata = merge_metadata_hehestl(
            root_metadata.as_deref(),
            &archive_id,
            file_paths,
            &options.compression.metadata_label(),
        );
        let items = build_create_items(&mut files, metadata);
        let stats = stats_from_items(&items);
        write_archive_atomic(output_path, &items, METHOD_ZSTD, options.compression)?;
        Ok(HeheCreateResult {
            archive_id,
            entry_count: stats.0,
            total_bytes: stats.1,
        })
    }

    pub fn create_from_archive(
        source_archive: &str,
        entry_paths: &[String],
        strip_prefix: Option<&str>,
        output_path: &str,
        options: HeheCreateOptions,
    ) -> AppResult<HeheCreateResult> {
        let toc = Self::read_toc(source_archive)?;
        let archive_id = Uuid::new_v4().to_string();
        let mut files: HashMap<String, Vec<u8>> = HashMap::new();
        let mut root_metadata: Option<String> = None;
        let mut ordered_paths: Vec<String> = Vec::new();

        for entry in &toc {
            if entry.path.ends_with('/') {
                continue;
            }
            let norm = normalize_entry_path(&entry.path);
            if !entry_matches_selection(&norm, entry_paths) {
                continue;
            }
            let mapped = remap_entry_path(&norm, strip_prefix)?;
            if mapped == METADATA_PATH {
                let bytes = Self::read_entry_bytes(source_archive, &entry.path)?;
                root_metadata = Some(String::from_utf8_lossy(&bytes).into_owned());
                continue;
            }
            if !ordered_paths.contains(&mapped) {
                ordered_paths.push(mapped.clone());
            }
            let data = Self::read_entry_bytes(source_archive, &entry.path)?;
            insert_unique(&mut files, mapped, data)?;
        }

        if options.convert_images_to_webp {
            let path_map = apply_webp_conversion(&mut files)?;
            for path in &mut ordered_paths {
                if let Some(mapped) = path_map.get(path) {
                    *path = mapped.clone();
                }
            }
        }

        let metadata = merge_metadata_hehestl(
            root_metadata.as_deref(),
            &archive_id,
            &[source_archive.to_string()],
            &options.compression.metadata_label(),
        );
        let mut items = vec![(METADATA_PATH.to_string(), metadata.into_bytes())];
        for path in ordered_paths {
            if let Some(data) = files.remove(&path) {
                items.push((path, data));
            }
        }
        let stats = stats_from_items(&items);
        write_archive_atomic(output_path, &items, METHOD_ZSTD, options.compression)?;
        Ok(HeheCreateResult {
            archive_id,
            entry_count: stats.0,
            total_bytes: stats.1,
        })
    }

    pub fn parse_archive_id_from_metadata(content: &str) -> Option<String> {
        for line in strip_utf8_bom(content).lines() {
            let line = line.trim();
            if let Some(rest) = line
                .strip_prefix("ArchiveId:")
                .or_else(|| line.strip_prefix("archive_id:"))
            {
                let id = rest.trim();
                if Uuid::parse_str(id).is_ok() {
                    return Some(id.to_string());
                }
            }
        }
        None
    }

    pub fn read_metadata(path: &str) -> AppResult<Option<String>> {
        if !Self::probe(path)? {
            return Ok(None);
        }
        match Self::read_entry_bytes(path, METADATA_PATH) {
            Ok(bytes) => Ok(Some(String::from_utf8_lossy(&bytes).into_owned())),
            Err(_) => Ok(None),
        }
    }

    fn load_toc_cached(path: &str) -> AppResult<CachedToc> {
        let meta = std::fs::metadata(path).map_err(|e| AppError::Archive(e.to_string()))?;
        let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let size = meta.len();
        let cache = TOC_CACHE.get_or_init(|| RwLock::new(HashMap::new()));

        if let Ok(guard) = cache.read() {
            if let Some(cached) = guard.get(path) {
                if cached.mtime == mtime && cached.size == size {
                    return Ok(cached.clone());
                }
            }
        }

        let entries = Self::read_toc_uncached(path)?;
        let by_path = entries
            .iter()
            .map(|e| (normalize_entry_path(&e.path), e.clone()))
            .collect();
        let cached = CachedToc {
            mtime,
            size,
            entries,
            by_path,
        };

        if let Ok(mut guard) = cache.write() {
            guard.insert(path.to_string(), cached.clone());
        }
        Ok(cached)
    }

    fn read_toc(path: &str) -> AppResult<Vec<TocEntry>> {
        Ok(Self::load_toc_cached(path)?.entries)
    }

    fn read_toc_uncached(path: &str) -> AppResult<Vec<TocEntry>> {
        let mut f = File::open(path).map_err(|e| AppError::Archive(e.to_string()))?;
        let header = read_header(&mut f)?;
        if header.magic != *MAGIC {
            return Err(AppError::Archive("invalid HEHE magic".into()));
        }
        if header.version != VERSION {
            return Err(AppError::Archive(format!(
                "unsupported HEHE version {}",
                header.version
            )));
        }
        f.seek(SeekFrom::Start(header.toc_offset))
            .map_err(|e| AppError::Archive(e.to_string()))?;
        let mut entries = Vec::with_capacity(header.toc_count as usize);
        for _ in 0..header.toc_count {
            entries.push(read_toc_entry(&mut f)?);
        }
        Ok(entries)
    }

    fn read_entry_raw(path: &str, entry: &TocEntry) -> AppResult<Vec<u8>> {
        let mut f = File::open(path).map_err(|e| AppError::Archive(e.to_string()))?;
        f.seek(SeekFrom::Start(entry.data_offset))
            .map_err(|e| AppError::Archive(e.to_string()))?;
        let mut comp = vec![0u8; entry.comp_size as usize];
        f.read_exact(&mut comp)
            .map_err(|e| AppError::Archive(e.to_string()))?;
        let raw = decompress(entry.method, &comp, entry.raw_size as usize)?;
        let mut hasher = Crc32::new();
        hasher.update(&raw);
        if hasher.finalize() != entry.crc32 {
            return Err(AppError::Archive(format!(
                "CRC mismatch for {}",
                entry.path
            )));
        }
        Ok(raw)
    }

    pub(crate) fn write_archive(
        output_path: &str,
        items: &[(String, Vec<u8>)],
        default_method: u8,
        compression: HeheCompression,
    ) -> AppResult<()> {
        if let Some(parent) = Path::new(output_path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut f = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(output_path)
            .map_err(|e| AppError::Archive(e.to_string()))?;
        f.write_all(&[0u8; HEADER_SIZE as usize])
            .map_err(|e| AppError::Archive(e.to_string()))?;
        let mut toc_entries = Vec::new();
        let mut offset = HEADER_SIZE;
        for (path, raw) in items {
            let method = compression_method_for_path(path, default_method, compression);
            let (method, comp) = compress(method, raw, compression)?;
            let mut hasher = Crc32::new();
            hasher.update(raw);
            toc_entries.push(TocEntry {
                path: path.clone(),
                method,
                crc32: hasher.finalize(),
                comp_size: comp.len() as u64,
                raw_size: raw.len() as u64,
                data_offset: offset,
            });
            f.write_all(&comp).map_err(|e| AppError::Archive(e.to_string()))?;
            offset += comp.len() as u64;
        }
        let toc_offset = offset;
        for entry in &toc_entries {
            write_toc_entry(&mut f, entry)?;
        }
        f.seek(SeekFrom::Start(0)).map_err(|e| AppError::Archive(e.to_string()))?;
        write_header(
            &mut f,
            FileHeader {
                magic: *MAGIC,
                version: VERSION,
                flags: 0,
                toc_offset,
                toc_count: toc_entries.len() as u32,
            },
        )?;
        Ok(())
    }
}

pub(crate) fn write_archive_atomic(
    output_path: &str,
    items: &[(String, Vec<u8>)],
    default_method: u8,
    compression: HeheCompression,
) -> AppResult<()> {
    let output = Path::new(output_path);
    let tmp_path = tmp_path_for(output_path);
    let result = (|| -> AppResult<()> {
        HeheFormat::write_archive(
            tmp_path
                .to_str()
                .ok_or_else(|| AppError::Archive("invalid tmp path".into()))?,
            items,
            default_method,
            compression,
        )?;
        verify_archive_entries(
            tmp_path
                .to_str()
                .ok_or_else(|| AppError::Archive("invalid tmp path".into()))?,
            items,
        )?;
        rename_with_retry(&tmp_path, output)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp_path);
    }
    result
}

fn collect_local_sources(paths: &[String]) -> AppResult<(HashMap<String, Vec<u8>>, Option<String>)> {
    let mut files: HashMap<String, Vec<u8>> = HashMap::new();
    let mut root_metadata: Option<String> = None;
    let multi_root = paths.len() > 1;

    for source in paths {
        let source_path = Path::new(source);
        if !source_path.exists() {
            return Err(AppError::Validation(format!("Путь не найден: {source}")));
        }

        let root_prefix = if multi_root {
            source_path
                .file_name()
                .map(|n| normalize_toc_path(&n.to_string_lossy()))
                .filter(|s| !s.is_empty())
        } else {
            None
        };

        if source_path.is_file() {
            let file_name = source_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .ok_or_else(|| AppError::Validation(format!("invalid path: {source}")))?;
            let rel = match &root_prefix {
                Some(prefix) => join_toc_path(prefix, &file_name),
                None => normalize_toc_path(&file_name),
            };
            if rel == METADATA_PATH {
                let content = std::fs::read_to_string(source_path)
                    .map_err(|e| AppError::Archive(e.to_string()))?;
                root_metadata = Some(content);
                continue;
            }
            let data = std::fs::read(source_path).map_err(|e| AppError::Archive(e.to_string()))?;
            insert_unique(&mut files, rel, data)?;
            continue;
        }

        if !source_path.is_dir() {
            return Err(AppError::Validation(format!("Неподдерживаемый путь: {source}")));
        }

        let mut queue: VecDeque<(PathBuf, String)> = VecDeque::new();
        queue.push_back((source_path.to_path_buf(), String::new()));

        while let Some((dir, rel_base)) = queue.pop_front() {
            let read_dir = std::fs::read_dir(&dir).map_err(|e| AppError::Archive(e.to_string()))?;
            for entry in read_dir {
                let entry = entry.map_err(|e| AppError::Archive(e.to_string()))?;
                let file_name = entry.file_name().to_string_lossy().to_string();
                let file_type = entry
                    .file_type()
                    .map_err(|e| AppError::Archive(e.to_string()))?;

                if file_type.is_dir() {
                    if should_skip_dir_name(&file_name) {
                        continue;
                    }
                    let child_rel = join_toc_path(&rel_base, &file_name);
                    queue.push_back((entry.path(), child_rel));
                    continue;
                }

                if !file_type.is_file() || should_skip_file_name(&file_name) {
                    continue;
                }

                let rel = join_toc_path(&rel_base, &file_name);
                let full_rel = match &root_prefix {
                    Some(prefix) => join_toc_path(prefix, &rel),
                    None => rel,
                };

                if full_rel == METADATA_PATH {
                    let content = std::fs::read_to_string(entry.path())
                        .map_err(|e| AppError::Archive(e.to_string()))?;
                    root_metadata = Some(content);
                    continue;
                }

                let data = std::fs::read(entry.path()).map_err(|e| AppError::Archive(e.to_string()))?;
                insert_unique(&mut files, full_rel, data)?;
            }
        }
    }

    Ok((files, root_metadata))
}

pub fn merge_metadata_hehestl(
    existing: Option<&str>,
    archive_id: &str,
    sources: &[String],
    compression_label: &str,
) -> String {
    enum PreservedUserLine {
        Field(String, String),
        Verbatim(String),
    }

    let mut user_lines: Vec<PreservedUserLine> = Vec::new();
    if let Some(raw) = existing {
        for line in strip_utf8_bom(raw).lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if !line.contains(':') {
                user_lines.push(PreservedUserLine::Verbatim(line.to_string()));
                continue;
            }
            if line.contains("(http") {
                user_lines.push(PreservedUserLine::Verbatim(line.to_string()));
                continue;
            }
            if line.contains(" | ") {
                user_lines.push(PreservedUserLine::Verbatim(line.to_string()));
                continue;
            }
            let Some((key, value)) = line.split_once(':') else {
                user_lines.push(PreservedUserLine::Verbatim(line.to_string()));
                continue;
            };
            let key = key.trim();
            if SYSTEM_METADATA_KEYS.contains(&key) {
                continue;
            }
            user_lines.push(PreservedUserLine::Field(key.to_string(), value.trim().to_string()));
        }
    }

    let created = Utc::now().to_rfc3339();
    let mut out = String::new();
    out.push_str("FormatVersion: 1\n");
    out.push_str(&format!("ArchiveId: {archive_id}\n"));
    out.push_str(&format!("Created: {created}\n"));
    out.push_str(&format!("Compression: {compression_label}\n"));
    if let Some(sources_line) = format_original_sources(sources) {
        out.push_str(&format!("OriginalSources: {sources_line}\n"));
    }
    if user_lines.is_empty() && existing.is_none() {
        out.push_str("Кат: Оригинальный | Cat: Original\nName:\nTags:\n");
    } else {
        for entry in user_lines {
            match entry {
                PreservedUserLine::Field(key, value) => {
                    out.push_str(&format!("{key}: {value}\n"));
                }
                PreservedUserLine::Verbatim(line) => {
                    out.push_str(&format!("{line}\n"));
                }
            }
        }
    }
    out
}

fn build_create_items(files: &mut HashMap<String, Vec<u8>>, metadata: String) -> Vec<(String, Vec<u8>)> {
    let mut paths: Vec<String> = files.keys().cloned().collect();
    paths.sort();
    let mut items = vec![(METADATA_PATH.to_string(), metadata.into_bytes())];
    for path in paths {
        if let Some(data) = files.remove(&path) {
            items.push((path, data));
        }
    }
    items
}

fn stats_from_items(items: &[(String, Vec<u8>)]) -> (u32, u64) {
    let entry_count = items.len() as u32;
    let total_bytes: u64 = items.iter().map(|(_, b)| b.len() as u64).sum();
    (entry_count, total_bytes)
}

fn verify_archive_entries(archive_path: &str, expected: &[(String, Vec<u8>)]) -> AppResult<()> {
    for (path, raw) in expected {
        let got = HeheFormat::read_entry_bytes(archive_path, path).map_err(|e| {
            AppError::Archive(format!(
                "Проверка целостности не пройдена: entry '{path}' — {e}"
            ))
        })?;
        if got.len() != raw.len() {
            return Err(AppError::Archive(format!(
                "Проверка целостности не пройдена: entry '{path}' — неверный размер"
            )));
        }
        let head_len = raw.len().min(64);
        if head_len > 0 && got[..head_len] != raw[..head_len] {
            return Err(AppError::Archive(format!(
                "Проверка целостности не пройдена: entry '{path}' — неверные данные"
            )));
        }
    }
    Ok(())
}

fn tmp_path_for(output_path: &str) -> PathBuf {
    let p = Path::new(output_path);
    let parent = p.parent().unwrap_or_else(|| Path::new("."));
    let stem = p
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("archive");
    parent.join(format!("{stem}.hehe.tmp"))
}

fn is_rename_lock_error(e: &std::io::Error) -> bool {
    e.kind() == std::io::ErrorKind::PermissionDenied || e.raw_os_error() == Some(5)
}

fn rename_with_retry(from: &Path, to: &Path) -> AppResult<()> {
    for attempt in 0..RENAME_MAX_ATTEMPTS {
        match std::fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(e) if is_rename_lock_error(&e) && attempt + 1 < RENAME_MAX_ATTEMPTS => {
                thread::sleep(Duration::from_millis(RENAME_RETRY_DELAY_MS));
            }
            Err(e) => {
                return Err(AppError::Archive(format!(
                    "Не удалось завершить запись архива: {e}. Файл может быть заблокирован антивирусом — повторите."
                )));
            }
        }
    }
    Err(AppError::Archive(
        "Не удалось завершить запись архива: файл заблокирован. Повторите.".into(),
    ))
}

fn entry_matches_selection(norm_path: &str, entry_paths: &[String]) -> bool {
    if entry_paths.is_empty() {
        return true;
    }
    entry_paths.iter().any(|sel| {
        let sel_norm = normalize_entry_path(sel);
        norm_path == sel_norm || norm_path.starts_with(&format!("{sel_norm}/"))
    })
}

fn remap_entry_path(original: &str, strip_prefix: Option<&str>) -> AppResult<String> {
    let norm = normalize_entry_path(original);
    let stripped = if let Some(prefix) = strip_prefix.filter(|p| !p.is_empty()) {
        let prefix_norm = normalize_entry_path(prefix);
        norm.strip_prefix(&prefix_norm)
            .map(|rest| rest.trim_start_matches('/'))
            .unwrap_or(norm.as_str())
    } else {
        norm.as_str()
    };
    let mapped = normalize_toc_path(stripped);
    if mapped.is_empty() {
        return Err(AppError::Validation(format!(
            "Пустой путь после strip: {original}"
        )));
    }
    Ok(mapped)
}

fn insert_unique(map: &mut HashMap<String, Vec<u8>>, path: String, data: Vec<u8>) -> AppResult<()> {
    if map.contains_key(&path) {
        return Err(AppError::Validation(format!("Коллизия имён: {path}")));
    }
    map.insert(path, data);
    Ok(())
}

fn join_toc_path(base: &str, name: &str) -> String {
    let name = normalize_toc_path(name);
    if base.is_empty() {
        name
    } else {
        format!("{}/{}", normalize_toc_path(base), name)
    }
}

fn normalize_toc_path(p: &str) -> String {
    p.replace('\\', "/")
        .split('/')
        .filter(|c| !c.is_empty() && *c != ".")
        .collect::<Vec<_>>()
        .join("/")
}

fn should_skip_file_name(name: &str) -> bool {
    SKIP_NAMES.contains(&name)
}

fn should_skip_dir_name(name: &str) -> bool {
    SKIP_DIR_NAMES.contains(&name)
}

fn strip_utf8_bom(s: &str) -> &str {
    s.strip_prefix('\u{FEFF}').unwrap_or(s)
}

fn format_original_sources(sources: &[String]) -> Option<String> {
    if sources.is_empty() {
        return None;
    }
    if sources.len() <= 3 {
        Some(sources.join("; "))
    } else {
        Some(format!("{}; …", sources[..3].join("; ")))
    }
}

struct FileHeader {
    magic: [u8; 4],
    version: u16,
    flags: u16,
    toc_offset: u64,
    toc_count: u32,
}

fn read_header(f: &mut File) -> AppResult<FileHeader> {
    let mut buf = [0u8; HEADER_SIZE as usize];
    f.read_exact(&mut buf).map_err(|e| AppError::Archive(e.to_string()))?;
    Ok(FileHeader {
        magic: buf[0..4].try_into().unwrap(),
        version: u16::from_le_bytes(buf[4..6].try_into().unwrap()),
        flags: u16::from_le_bytes(buf[6..8].try_into().unwrap()),
        toc_offset: u64::from_le_bytes(buf[8..16].try_into().unwrap()),
        toc_count: u32::from_le_bytes(buf[16..20].try_into().unwrap()),
    })
}

fn write_header(f: &mut File, h: FileHeader) -> AppResult<()> {
    let mut buf = [0u8; HEADER_SIZE as usize];
    buf[0..4].copy_from_slice(&h.magic);
    buf[4..6].copy_from_slice(&h.version.to_le_bytes());
    buf[6..8].copy_from_slice(&h.flags.to_le_bytes());
    buf[8..16].copy_from_slice(&h.toc_offset.to_le_bytes());
    buf[16..20].copy_from_slice(&h.toc_count.to_le_bytes());
    f.write_all(&buf).map_err(|e| AppError::Archive(e.to_string()))?;
    Ok(())
}

fn read_toc_entry(f: &mut File) -> AppResult<TocEntry> {
    let mut len_buf = [0u8; 2];
    f.read_exact(&mut len_buf).map_err(|e| AppError::Archive(e.to_string()))?;
    let path_len = u16::from_le_bytes(len_buf) as usize;
    let mut path_bytes = vec![0u8; path_len];
    f.read_exact(&mut path_bytes).map_err(|e| AppError::Archive(e.to_string()))?;
    let path = String::from_utf8(path_bytes).map_err(|e| AppError::Archive(e.to_string()))?;
    let mut method_buf = [0u8; 1];
    f.read_exact(&mut method_buf).map_err(|e| AppError::Archive(e.to_string()))?;
    let mut crc_buf = [0u8; 4];
    f.read_exact(&mut crc_buf).map_err(|e| AppError::Archive(e.to_string()))?;
    let mut cs_buf = [0u8; 8];
    f.read_exact(&mut cs_buf).map_err(|e| AppError::Archive(e.to_string()))?;
    let mut rs_buf = [0u8; 8];
    f.read_exact(&mut rs_buf).map_err(|e| AppError::Archive(e.to_string()))?;
    let mut do_buf = [0u8; 8];
    f.read_exact(&mut do_buf).map_err(|e| AppError::Archive(e.to_string()))?;
    Ok(TocEntry {
        path,
        method: method_buf[0],
        crc32: u32::from_le_bytes(crc_buf),
        comp_size: u64::from_le_bytes(cs_buf),
        raw_size: u64::from_le_bytes(rs_buf),
        data_offset: u64::from_le_bytes(do_buf),
    })
}

fn write_toc_entry(f: &mut File, e: &TocEntry) -> AppResult<()> {
    let path_bytes = e.path.as_bytes();
    if path_bytes.len() > u16::MAX as usize {
        return Err(AppError::Archive("path too long".into()));
    }
    f.write_all(&(path_bytes.len() as u16).to_le_bytes())
        .map_err(|e| AppError::Archive(e.to_string()))?;
    f.write_all(path_bytes).map_err(|e| AppError::Archive(e.to_string()))?;
    f.write_all(&[e.method]).map_err(|e| AppError::Archive(e.to_string()))?;
    f.write_all(&e.crc32.to_le_bytes()).map_err(|e| AppError::Archive(e.to_string()))?;
    f.write_all(&e.comp_size.to_le_bytes()).map_err(|e| AppError::Archive(e.to_string()))?;
    f.write_all(&e.raw_size.to_le_bytes()).map_err(|e| AppError::Archive(e.to_string()))?;
    f.write_all(&e.data_offset.to_le_bytes()).map_err(|e| AppError::Archive(e.to_string()))?;
    Ok(())
}

fn compression_method_for_path(path: &str, default_method: u8, compression: HeheCompression) -> u8 {
    if !compression.compress_stl {
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if matches!(ext.as_str(), "stl" | "obj") {
            return METHOD_STORE;
        }
    }
    default_method
}

fn encode_zstd(raw: &[u8], compression: HeheCompression) -> AppResult<Vec<u8>> {
    if let Some(window_log) = compression.window_log {
        use zstd::bulk::Compressor;
        use zstd::zstd_safe::CParameter;
        let mut enc = Compressor::new(compression.zstd_level)
            .map_err(|e| AppError::Archive(e.to_string()))?;
        enc.set_parameter(CParameter::WindowLog(window_log))
            .map_err(|e| AppError::Archive(e.to_string()))?;
        enc.compress(raw)
            .map_err(|e| AppError::Archive(e.to_string()))
    } else {
        zstd::stream::encode_all(raw, compression.zstd_level)
            .map_err(|e| AppError::Archive(e.to_string()))
    }
}

fn compress(method: u8, raw: &[u8], compression: HeheCompression) -> AppResult<(u8, Vec<u8>)> {
    match method {
        METHOD_STORE => Ok((METHOD_STORE, raw.to_vec())),
        METHOD_DEFLATE => {
            use flate2::write::ZlibEncoder;
            use flate2::Compression;
            use std::io::Write;
            let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
            enc.write_all(raw).map_err(|e| AppError::Archive(e.to_string()))?;
            let comp = enc.finish().map_err(|e| AppError::Archive(e.to_string()))?;
            if comp.len() >= raw.len() {
                return Ok((METHOD_STORE, raw.to_vec()));
            }
            Ok((METHOD_DEFLATE, comp))
        }
        METHOD_ZSTD | _ => {
            let comp = encode_zstd(raw, compression)?;
            if comp.len() >= raw.len() {
                return Ok((METHOD_STORE, raw.to_vec()));
            }
            Ok((METHOD_ZSTD, comp))
        }
    }
}

fn collect_target_entries<'a>(
    cached: &'a CachedToc,
    filter_set: &HashSet<String>,
) -> Vec<&'a TocEntry> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();

    for f in filter_set {
        if let Some(entry) = cached.by_path.get(f) {
            if entry.path.ends_with('/') {
                continue;
            }
            let norm = normalize_entry_path(&entry.path);
            if seen.insert(norm) {
                out.push(entry);
            }
        }
    }

    let all_direct = filter_set.iter().all(|f| {
        cached
            .by_path
            .get(f)
            .is_some_and(|e| !e.path.ends_with('/'))
    });

    if all_direct {
        return out;
    }

    for entry in &cached.entries {
        if entry.path.ends_with('/') {
            continue;
        }
        let norm = normalize_entry_path(&entry.path);
        if seen.contains(&norm) {
            continue;
        }
        let matches = filter_set.contains(&norm)
            || filter_set
                .iter()
                .any(|f| norm.starts_with(&format!("{f}/")));
        if matches && seen.insert(norm) {
            out.push(entry);
        }
    }
    out
}

fn extract_entry_to_file(
    archive: &mut File,
    entry: &TocEntry,
    dest: &Path,
) -> AppResult<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError::Archive(e.to_string()))?;
    }
    archive
        .seek(SeekFrom::Start(entry.data_offset))
        .map_err(|e| AppError::Archive(e.to_string()))?;

    let mut hasher = Crc32::new();
    let file = File::create(dest).map_err(|e| AppError::Archive(e.to_string()))?;
    let mut out = BufWriter::with_capacity(256 * 1024, file);
    let mut limited = archive.take(entry.comp_size);
    const READ_BUF: usize = 512 * 1024;
    let mut buf = vec![0u8; READ_BUF];

    match entry.method {
        METHOD_STORE => {
            loop {
                let n = limited
                    .read(&mut buf)
                    .map_err(|e| AppError::Archive(e.to_string()))?;
                if n == 0 {
                    break;
                }
                hasher.update(&buf[..n]);
                out.write_all(&buf[..n])
                    .map_err(|e| AppError::Archive(e.to_string()))?;
            }
        }
        METHOD_DEFLATE => {
            use flate2::read::ZlibDecoder;
            let mut dec = ZlibDecoder::new(&mut limited);
            loop {
                let n = dec
                    .read(&mut buf)
                    .map_err(|e| AppError::Archive(e.to_string()))?;
                if n == 0 {
                    break;
                }
                hasher.update(&buf[..n]);
                out.write_all(&buf[..n])
                    .map_err(|e| AppError::Archive(e.to_string()))?;
            }
        }
        METHOD_ZSTD => {
            let mut dec = zstd::stream::read::Decoder::new(&mut limited)
                .map_err(|e| AppError::Archive(e.to_string()))?;
            loop {
                let n = dec
                    .read(&mut buf)
                    .map_err(|e| AppError::Archive(e.to_string()))?;
                if n == 0 {
                    break;
                }
                hasher.update(&buf[..n]);
                out.write_all(&buf[..n])
                    .map_err(|e| AppError::Archive(e.to_string()))?;
            }
        }
        _ => {
            return Err(AppError::Archive(format!(
                "unknown method {}",
                entry.method
            )));
        }
    }

    out.flush().map_err(|e| AppError::Archive(e.to_string()))?;

    if hasher.finalize() != entry.crc32 {
        let _ = std::fs::remove_file(dest);
        return Err(AppError::Archive(format!(
            "CRC mismatch for {}",
            entry.path
        )));
    }
    Ok(())
}

fn decompress(method: u8, comp: &[u8], raw_size: usize) -> AppResult<Vec<u8>> {
    match method {
        METHOD_STORE => Ok(comp.to_vec()),
        METHOD_DEFLATE => {
            use flate2::read::ZlibDecoder;
            use std::io::Read;
            let mut dec = ZlibDecoder::new(comp);
            let mut out = Vec::with_capacity(raw_size);
            dec.read_to_end(&mut out)
                .map_err(|e| AppError::Archive(e.to_string()))?;
            Ok(out)
        }
        METHOD_ZSTD => {
            zstd::stream::decode_all(comp).map_err(|e| AppError::Archive(e.to_string()))
        }
        _ => Err(AppError::Archive(format!("unknown method {method}"))),
    }
}

fn normalize_entry_path(p: &str) -> String {
    p.replace('\\', "/").trim_end_matches('/').to_string()
}

fn entry_dto(path: &str, size: u64, packed: u64) -> ArchiveEntryDto {
    let is_dir = path.ends_with('/');
    let name = path
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .to_string();
    let extension = if is_dir {
        String::new()
    } else {
        name.rsplit('.')
            .next()
            .filter(|_| name.contains('.'))
            .unwrap_or("")
            .to_ascii_lowercase()
    };
    ArchiveEntryDto {
        path: path.to_string(),
        name,
        size,
        packed_size: packed,
        modified: None,
        is_dir,
        extension,
    }
}

pub fn default_metadata_hehestl(archive_id: &str) -> String {
    merge_metadata_hehestl(
        None,
        archive_id,
        &[],
        &HeheCompression::balanced().metadata_label(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn stl_created_as_store() {
        let dir = TempDir::new().unwrap();
        let src = dir.path().join("part.stl");
        fs::write(&src, b"solid test cube").unwrap();
        let out = dir.path().join("store-stl.hehe");
        HeheFormat::create(&out.to_string_lossy(), &[src.to_string_lossy().into_owned()], HeheCreateOptions::from_api(None, None))
            .unwrap();
        let list = HeheFormat::list(&out.to_string_lossy()).unwrap();
        let stl = list.iter().find(|e| e.name == "part.stl").expect("stl");
        assert_eq!(stl.size, stl.packed_size);
    }

    #[test]
    fn targeted_extract_from_large_toc() {
        use std::time::Instant;
        let dir = TempDir::new().unwrap();
        let mut items: Vec<(String, Vec<u8>)> = (0..1000)
            .map(|i| (format!("bulk/f{i:04}.bin"), vec![0u8; 16]))
            .collect();
        items.push(("target.stl".to_string(), b"solid target".to_vec()));
        items.push((METADATA_PATH.to_string(), b"ArchiveId: x\n".to_vec()));
        HeheFormat::write_archive(
            &dir.path().join("big.hehe").to_string_lossy(),
            &items,
            METHOD_ZSTD,
            HeheCompression::balanced(),
        )
            .unwrap();
        let dest = dir.path().join("out");
        let start = Instant::now();
        let result = HeheFormat::extract(
            &dir.path().join("big.hehe").to_string_lossy(),
            &dest.to_string_lossy(),
            &["target.stl".to_string()],
            false,
        )
        .unwrap();
        assert_eq!(result.written.len(), 1);
        assert_eq!(fs::read(&result.written[0]).unwrap(), b"solid target");
        assert!(
            start.elapsed().as_secs() < 5,
            "targeted extract took too long: {:?}",
            start.elapsed()
        );
    }

    #[test]
    fn streaming_extract_matches_read_entry_bytes() {
        let dir = TempDir::new().unwrap();
        let src = dir.path().join("part.stl");
        let payload = b"solid streaming test";
        fs::write(&src, payload).unwrap();
        let out = dir.path().join("stream.hehe");
        HeheFormat::create(&out.to_string_lossy(), &[src.to_string_lossy().into_owned()], HeheCreateOptions::from_api(None, None))
            .unwrap();
        let dest = dir.path().join("extracted");
        let result = HeheFormat::extract(
            &out.to_string_lossy(),
            &dest.to_string_lossy(),
            &["part.stl".to_string()],
            false,
        )
        .unwrap();
        assert_eq!(result.written.len(), 1);
        let from_disk = fs::read(&result.written[0]).unwrap();
        let from_archive =
            HeheFormat::read_entry_bytes(&out.to_string_lossy(), "part.stl").unwrap();
        assert_eq!(from_disk, from_archive);
        assert_eq!(from_disk, payload);
    }

    #[test]
    fn roundtrip_zstd() {
        let dir = TempDir::new().unwrap();
        let src = dir.path().join("part.stl");
        fs::write(&src, b"solid test cube").unwrap();
        let out = dir.path().join("test.hehe");
        let result =
            HeheFormat::create(&out.to_string_lossy(), &[src.to_string_lossy().into_owned()], HeheCreateOptions::from_api(None, None))
                .unwrap();
        assert!(Uuid::parse_str(&result.archive_id).is_ok());
        assert!(HeheFormat::probe(&out.to_string_lossy()).unwrap());
        let list = HeheFormat::list(&out.to_string_lossy()).unwrap();
        assert!(list.iter().any(|e| e.path == METADATA_PATH));
        assert!(list.iter().any(|e| e.name == "part.stl"));
        let meta = HeheFormat::read_entry_bytes(&out.to_string_lossy(), METADATA_PATH).unwrap();
        let meta_str = String::from_utf8_lossy(&meta);
        assert_eq!(
            HeheFormat::parse_archive_id_from_metadata(&meta_str),
            Some(result.archive_id)
        );
        assert!(meta_str.contains("Compression: zstd:12"));
        let data = HeheFormat::read_entry_bytes(&out.to_string_lossy(), "part.stl").unwrap();
        assert_eq!(data, b"solid test cube");
    }

    #[test]
    fn roundtrip_store_and_deflate() {
        let dir = TempDir::new().unwrap();
        let out_store = dir.path().join("store.hehe");
        let out_deflate = dir.path().join("deflate.hehe");
        let items = vec![("hello.txt".to_string(), b"hello".to_vec())];
        HeheFormat::write_archive(
            &out_store.to_string_lossy(),
            &items,
            METHOD_STORE,
            HeheCompression::balanced(),
        )
        .unwrap();
        HeheFormat::write_archive(
            &out_deflate.to_string_lossy(),
            &items,
            METHOD_DEFLATE,
            HeheCompression::balanced(),
        ).unwrap();
        for path in [&out_store, &out_deflate] {
            let bytes = HeheFormat::read_entry_bytes(&path.to_string_lossy(), "hello.txt").unwrap();
            assert_eq!(bytes, b"hello");
        }
    }

    #[test]
    fn fuzzish_paths() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("fuzz.hehe");
        let big = vec![0u8; 64 * 1024];
        let items = vec![
            ("empty.bin".to_string(), vec![]),
            ("nested/deep/file.bin".to_string(), big.clone()),
        ];
        HeheFormat::write_archive(
            &out.to_string_lossy(),
            &items,
            METHOD_ZSTD,
            HeheCompression::balanced(),
        )
        .unwrap();
        let got =
            HeheFormat::read_entry_bytes(&out.to_string_lossy(), "nested/deep/file.bin").unwrap();
        assert_eq!(got.len(), big.len());
    }

    #[test]
    fn create_from_directory() {
        let dir = TempDir::new().unwrap();
        let root = dir.path().join("project");
        fs::create_dir_all(root.join("refs")).unwrap();
        fs::write(root.join("part.stl"), b"solid x").unwrap();
        fs::write(root.join("refs/photo.png"), b"png").unwrap();
        fs::write(
            root.join("metadata.hehestl"),
            "Tags: keep-me\nArchiveId: old\n",
        )
        .unwrap();
        let out = dir.path().join("out.hehe");
        let result = HeheFormat::create(&out.to_string_lossy(), &[root.to_string_lossy().into_owned()], HeheCreateOptions::from_api(None, None))
            .unwrap();
        let meta_bytes =
            HeheFormat::read_entry_bytes(&out.to_string_lossy(), METADATA_PATH).unwrap();
        let meta = String::from_utf8_lossy(&meta_bytes);
        assert!(meta.contains("Tags: keep-me"));
        assert!(!meta.contains("old"));
        assert_eq!(
            HeheFormat::parse_archive_id_from_metadata(&meta),
            Some(result.archive_id)
        );
        assert!(
            HeheFormat::read_entry_bytes(&out.to_string_lossy(), "part.stl")
                .unwrap()
                .eq(b"solid x")
        );
        assert!(
            HeheFormat::read_entry_bytes(&out.to_string_lossy(), "refs/photo.png")
                .unwrap()
                .eq(b"png")
        );
    }

    #[test]
    fn create_empty_directory() {
        let dir = TempDir::new().unwrap();
        let root = dir.path().join("empty");
        fs::create_dir_all(&root).unwrap();
        let out = dir.path().join("empty.hehe");
        let result = HeheFormat::create(&out.to_string_lossy(), &[root.to_string_lossy().into_owned()], HeheCreateOptions::from_api(None, None))
            .unwrap();
        assert_eq!(result.entry_count, 1);
        let meta = HeheFormat::read_metadata(&out.to_string_lossy()).unwrap();
        assert!(meta.is_some());
    }

    #[test]
    fn create_cyrillic_paths() {
        let dir = TempDir::new().unwrap();
        let root = dir.path().join("проект");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("деталь.stl"), b"stl").unwrap();
        let out = dir.path().join("cyr.hehe");
        HeheFormat::create(&out.to_string_lossy(), &[root.to_string_lossy().into_owned()], HeheCreateOptions::from_api(None, None)).unwrap();
        let bytes =
            HeheFormat::read_entry_bytes(&out.to_string_lossy(), "деталь.stl").unwrap();
        assert_eq!(bytes, b"stl");
    }

    #[test]
    fn create_deep_nesting() {
        let dir = TempDir::new().unwrap();
        let mut path = dir.path().to_path_buf();
        for i in 0..25 {
            path = path.join(format!("level{i}"));
            fs::create_dir_all(&path).unwrap();
        }
        fs::write(path.join("deep.bin"), b"ok").unwrap();
        let out = dir.path().join("deep.hehe");
        HeheFormat::create(&out.to_string_lossy(), &[dir.path().to_string_lossy().into_owned()], HeheCreateOptions::from_api(None, None))
            .unwrap();
        let rel = (0..25)
            .map(|i| format!("level{i}"))
            .collect::<Vec<_>>()
            .join("/");
        let entry = format!("{rel}/deep.bin");
        assert_eq!(
            HeheFormat::read_entry_bytes(&out.to_string_lossy(), &entry).unwrap(),
            b"ok"
        );
    }

    #[test]
    fn merge_metadata_strips_bom() {
        let merged = merge_metadata_hehestl(
            Some("\u{FEFF}Tags: saved\nName: test\n"),
            "550e8400-e29b-41d4-a716-446655440000",
            &[],
            "zstd:12",
        );
        assert!(merged.contains("Tags: saved"));
        assert!(merged.contains("Name: test"));
        assert!(merged.contains("FormatVersion: 1"));
    }

    #[test]
    fn skip_dirs() {
        let dir = TempDir::new().unwrap();
        let root = dir.path().join("src");
        fs::create_dir_all(root.join(".git/objects")).unwrap();
        fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
        fs::write(root.join("part.stl"), b"x").unwrap();
        fs::write(root.join("node_modules/pkg/junk.txt"), b"j").unwrap();
        let out = dir.path().join("skip.hehe");
        HeheFormat::create(&out.to_string_lossy(), &[root.to_string_lossy().into_owned()], HeheCreateOptions::from_api(None, None)).unwrap();
        let list = HeheFormat::list(&out.to_string_lossy()).unwrap();
        assert!(list.iter().any(|e| e.path == "part.stl"));
        assert!(!list.iter().any(|e| e.path.contains("node_modules")));
        assert!(!list.iter().any(|e| e.path.contains(".git")));
    }

    #[test]
    fn create_from_archive_subset() {
        let dir = TempDir::new().unwrap();
        let src_dir = dir.path().join("src");
        fs::create_dir_all(src_dir.join("sub")).unwrap();
        fs::write(src_dir.join("a.stl"), b"a").unwrap();
        fs::write(src_dir.join("sub/b.stl"), b"b").unwrap();
        let source = dir.path().join("source.hehe");
        HeheFormat::create(
            &source.to_string_lossy(),
            &[src_dir.to_string_lossy().into_owned()],
            HeheCreateOptions::from_api(None, None),
        )
        .unwrap();
        let out = dir.path().join("repack.hehe");
        HeheFormat::create_from_archive(
            &source.to_string_lossy(),
            &["sub/".to_string()],
            Some("sub"),
            &out.to_string_lossy(),
            HeheCreateOptions::from_api(None, None),
        )
        .unwrap();
        assert!(
            HeheFormat::read_entry_bytes(&out.to_string_lossy(), "b.stl")
                .unwrap()
                .eq(b"b")
        );
        let list = HeheFormat::list(&out.to_string_lossy()).unwrap();
        assert!(!list.iter().any(|e| e.path == "a.stl"));
    }

    #[test]
    fn create_converts_png_to_webp_when_enabled() {
        use image::{ImageBuffer, Rgb, RgbImage};
        use std::io::Cursor;
        let dir = TempDir::new().unwrap();
        let root = dir.path().join("pack");
        fs::create_dir_all(&root).unwrap();
        let mut img: RgbImage = ImageBuffer::new(48, 48);
        for p in img.pixels_mut() {
            *p = Rgb([40, 120, 200]);
        }
        let png_path = root.join("ref.png");
        let mut png_buf = Vec::new();
        img.write_to(&mut Cursor::new(&mut png_buf), image::ImageFormat::Png)
            .unwrap();
        fs::write(&png_path, &png_buf).unwrap();
        let out = dir.path().join("webp.hehe");
        HeheFormat::create(
            &out.to_string_lossy(),
            &[root.to_string_lossy().into_owned()],
            HeheCreateOptions::from_api(None, Some(true)),
        )
        .unwrap();
        let list = HeheFormat::list(&out.to_string_lossy()).unwrap();
        assert!(list.iter().any(|e| e.path == "ref.webp"));
        assert!(!list.iter().any(|e| e.path == "ref.png"));
    }

    #[test]
    fn multi_root_avoids_basename_collision() {
        let dir = TempDir::new().unwrap();
        let root = dir.path().join("src");
        fs::create_dir_all(root.join("a")).unwrap();
        fs::create_dir_all(root.join("b")).unwrap();
        fs::write(root.join("a/x.txt"), b"1").unwrap();
        fs::write(root.join("b/x.txt"), b"2").unwrap();
        let (files, _) = collect_local_sources(&[
            root.join("a").to_string_lossy().into_owned(),
            root.join("b").to_string_lossy().into_owned(),
        ])
        .unwrap();
        assert_eq!(files.get("a/x.txt").map(|b| b.as_slice()), Some(b"1".as_ref()));
        assert_eq!(files.get("b/x.txt").map(|b| b.as_slice()), Some(b"2".as_ref()));
    }

    #[test]
    fn insert_unique_reports_collision() {
        let mut map = HashMap::new();
        insert_unique(&mut map, "foo.txt".into(), b"a".to_vec()).unwrap();
        let err = insert_unique(&mut map, "foo.txt".into(), b"b".to_vec()).unwrap_err();
        assert!(err.to_string().contains("Коллизия"));
    }
}
