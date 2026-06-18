mod archive;
mod auth;
mod clipboard;
mod error;
mod sync;
mod workflow;

use archive::adapter::ensure_hehe_extension;
use archive::archive_service::ArchiveService;
use archive::extract_cache;
use archive::hehe_format::HeheCreateResult;
use archive::hehe_format::HeheFormat;
use archive::preview_cache::{mime_from_path, PreviewCache};
use archive::thumb_disk_cache;
use archive::seven_zip::{ArchiveEntryDto, ExtractOptions, OverwriteMode};
use archive::seven_zip::normalize_archive_path;
use archive::temp_session::{self, create_session_dir, drop_session};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use error::{AppError, AppResult};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tauri::Emitter;
use tauri::Manager;
use auth::{
    clear_session, has_session, start_heron_login, AuthLoginResult,
};
use sync::{
    HestiaSyncClient, ManifestEntryDto, RemoteArchiveLinkPayload, SyncConfig, SyncQueueItem,
};
use workflow::db::WorkflowDb;
use workflow::sidecar::{ensure_sidecar, try_restore_statuses};
use workflow::types::{ActionLogEntry, EntryStatusMap, WorkflowStatus};

struct SessionInfo {
    session_id: String,
    #[allow(dead_code)]
    archive_hash: String,
    last_access: Instant,
}

const SESSION_IDLE_TTL_SECS: u64 = 30 * 60;

struct AppState {
    archive_service: Arc<ArchiveService>,
    preview_cache: PreviewCache,
    db: WorkflowDb,
    sessions: RwLock<HashMap<String, SessionInfo>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OpenArchiveSessionResult {
    archive_id: String,
    metadata_warning: Option<String>,
    has_hehestl: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PreviewBytesResult {
    base64: String,
    mime: String,
}

fn resolve_preview_bytes(
    state: &AppState,
    archive_id: &str,
    entry_path: &str,
) -> AppResult<(std::sync::Arc<[u8]>, String, bool)> {
    if let Some((bytes, mime)) = state.preview_cache.get(archive_id, entry_path) {
        return Ok((bytes, mime, true));
    }

    let archive_path = state
        .db
        .find_path_by_archive_id(archive_id)?
        .ok_or_else(|| AppError::Archive("archive not found".into()))?;
    let archive_hash = extract_cache::archive_hash(&archive_path)?;

    if let Some(disk_bytes) = thumb_disk_cache::read_if_exists(&archive_hash, entry_path) {
        let mime = "image/webp".to_string();
        state.preview_cache.put(
            archive_id,
            entry_path,
            disk_bytes.clone(),
            mime.clone(),
        );
        return Ok((std::sync::Arc::from(disk_bytes.into_boxed_slice()), mime, true));
    }

    let bytes = state
        .archive_service
        .read_entry_bytes(&archive_path, entry_path)?;
    let mime = mime_from_path(entry_path).to_string();
    let (bytes, mime) = match archive::preview_thumb::to_webp_thumb(&bytes, &mime) {
        Ok(Some((webp, webp_mime))) => {
            let _ = thumb_disk_cache::write(&archive_hash, entry_path, &webp);
            (webp, webp_mime)
        }
        _ => (bytes, mime),
    };
    state
        .preview_cache
        .put(archive_id, entry_path, bytes.clone(), mime.clone());
    Ok((std::sync::Arc::from(bytes.into_boxed_slice()), mime, false))
}

#[tauri::command]
fn read_preview_bytes(
    archive_id: String,
    entry_path: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<PreviewBytesResult> {
    let _span = tracing::info_span!(
        "preview_bytes",
        archive_id = %archive_id,
        entry = %entry_path
    )
    .entered();
    let start = Instant::now();

    let (bytes, mime, cache_hit) =
        resolve_preview_bytes(&state, &archive_id, &entry_path)?;
    tracing::info!(
        elapsed_ms = start.elapsed().as_millis(),
        cache_hit,
        "preview_bytes"
    );
    Ok(PreviewBytesResult {
        base64: STANDARD.encode(bytes.as_ref()),
        mime,
    })
}

struct ArchiveOpenProbe {
    is_hehe: bool,
    hehe_metadata: Option<String>,
    has_hehestl: bool,
    metadata_warning: Option<String>,
}

fn probe_archive_open_meta(path: &str, service: &ArchiveService) -> AppResult<ArchiveOpenProbe> {
    let _span = tracing::info_span!("archive_open_probe", path = %path).entered();
    let start = Instant::now();
    let mut probe = ArchiveOpenProbe {
        is_hehe: false,
        hehe_metadata: None,
        has_hehestl: false,
        metadata_warning: None,
    };

    if HeheFormat::probe(path)? {
        probe.is_hehe = true;
        match HeheFormat::read_metadata(path)? {
            Some(meta) => {
                probe.has_hehestl = true;
                probe.hehe_metadata = Some(meta);
            }
            None => {
                probe.metadata_warning = Some("архив .hehe без metadata.hehestl".into());
            }
        }
    } else {
        probe.has_hehestl = service.read_hehestl(path)?.is_some();
    }

    tracing::info!(
        elapsed_ms = start.elapsed().as_millis(),
        is_hehe = probe.is_hehe,
        has_hehestl = probe.has_hehestl,
        "archive_open_probe"
    );
    Ok(probe)
}

fn finalize_open_session(
    normalized: &str,
    db: &WorkflowDb,
    probe: ArchiveOpenProbe,
) -> AppResult<OpenArchiveSessionResult> {
    let _span = tracing::info_span!("archive_open_finalize", path = %normalized).entered();
    db.touch_recent(normalized)?;

    if probe.is_hehe {
        if let Some(meta) = probe.hehe_metadata {
            if let Some(id) = HeheFormat::parse_archive_id_from_metadata(&meta) {
                db.register_archive(normalized, &id, None)?;
                let _ = db.log_action(&id, Some(normalized), "open", None, None, None, None);
                return Ok(OpenArchiveSessionResult {
                    archive_id: id,
                    metadata_warning: probe.metadata_warning,
                    has_hehestl: probe.has_hehestl,
                });
            }
        }
        let id = ensure_sidecar(normalized, db)?;
        let _ = db.log_action(&id, Some(normalized), "open", None, None, None, None);
        return Ok(OpenArchiveSessionResult {
            archive_id: id,
            metadata_warning: probe
                .metadata_warning
                .or_else(|| Some("metadata.hehestl без валидного ArchiveId".into())),
            has_hehestl: probe.has_hehestl,
        });
    }

    let id = ensure_sidecar(normalized, db)?;
    let _ = db.log_action(&id, Some(normalized), "open", None, None, None, None);
    Ok(OpenArchiveSessionResult {
        archive_id: id,
        metadata_warning: probe.metadata_warning,
        has_hehestl: probe.has_hehestl,
    })
}

#[tauri::command]
async fn list_archive_entries(
    archive_path: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<Vec<ArchiveEntryDto>> {
    let normalized = normalize_archive_path(&archive_path)?;
    let service = state.archive_service.clone();
    tauri::async_runtime::spawn_blocking(move || service.list(&normalized))
        .await
        .map_err(|e| AppError::Archive(e.to_string()))?
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PaginatedEntriesResult {
    entries: Vec<ArchiveEntryDto>,
    total_count: usize,
    offset: usize,
    limit: usize,
}

#[tauri::command]
async fn list_archive_entries_paginated(
    archive_path: String,
    offset: usize,
    limit: usize,
    state: tauri::State<'_, AppState>,
) -> AppResult<PaginatedEntriesResult> {
    let normalized = normalize_archive_path(&archive_path)?;
    let service = state.archive_service.clone();
    let (entries, total_count) = tauri::async_runtime::spawn_blocking(move || {
        service.list_paginated(&normalized, offset, limit)
    })
    .await
    .map_err(|e| AppError::Archive(e.to_string()))??;
    Ok(PaginatedEntriesResult {
        entries,
        total_count,
        offset,
        limit,
    })
}

#[tauri::command]
fn probe_archive(path: String, state: tauri::State<'_, AppState>) -> AppResult<bool> {
    state.archive_service.probe(&path)
}

#[tauri::command]
async fn extract_archive(
    archive_path: String,
    destination: String,
    entries: Vec<String>,
    preserve_paths: bool,
    overwrite: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<Vec<String>> {
    let normalized = normalize_archive_path(&archive_path)?;
    let overwrite_mode = match overwrite.as_str() {
        "skip" => OverwriteMode::Skip,
        "replace" => OverwriteMode::Replace,
        _ => OverwriteMode::Ask,
    };
    let options = ExtractOptions {
        preserve_paths,
        overwrite: overwrite_mode,
        ..ExtractOptions::default()
    };
    let service = state.archive_service.clone();
    let dest = destination;
    let entry_list = entries;
    tauri::async_runtime::spawn_blocking(move || {
        let _span = tracing::info_span!(
            "archive_extract",
            path = %normalized,
            count = entry_list.len()
        )
        .entered();
        let start = Instant::now();
        let written = service
            .extract(&normalized, &dest, &entry_list, &options)?
            .written;
        tracing::info!(
            elapsed_ms = start.elapsed().as_millis(),
            written = written.len(),
            "archive_extract"
        );
        Ok(written)
    })
    .await
    .map_err(|e| AppError::Archive(e.to_string()))?
}

#[tauri::command]
fn normalize_path(path: String) -> AppResult<String> {
    normalize_archive_path(&path)
}

#[tauri::command]
async fn open_archive_session(
    archive_path: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<OpenArchiveSessionResult> {
    let normalized = normalize_archive_path(&archive_path)?;
    let service = state.archive_service.clone();
    let path_for_probe = normalized.clone();

    let probe = tauri::async_runtime::spawn_blocking(move || {
        probe_archive_open_meta(&path_for_probe, &service)
    })
    .await
    .map_err(|e| AppError::Archive(e.to_string()))??;

    finalize_open_session(&normalized, &state.db, probe)
}

#[tauri::command]
fn try_restore_archive_statuses(
    archive_path: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<Option<u32>> {
    let normalized = normalize_archive_path(&archive_path)?;
    try_restore_statuses(&normalized, &state.db)
}

#[tauri::command]
fn get_workflow_statuses(state: tauri::State<'_, AppState>) -> AppResult<Vec<WorkflowStatus>> {
    state.db.list_statuses()
}

#[tauri::command]
fn create_workflow_status(
    label: String,
    color: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<WorkflowStatus> {
    state.db.create_status(&label, &color)
}

#[tauri::command]
fn update_workflow_status(
    id: String,
    label: String,
    color: String,
    sort_order: i64,
    state: tauri::State<'_, AppState>,
) -> AppResult<WorkflowStatus> {
    state.db.update_status(&id, &label, &color, sort_order)
}

#[tauri::command]
fn delete_workflow_status(id: String, state: tauri::State<'_, AppState>) -> AppResult<()> {
    state.db.delete_status(&id)
}

#[tauri::command]
fn get_entry_statuses(
    archive_path: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<EntryStatusMap> {
    let normalized = normalize_archive_path(&archive_path)?;
    state.db.get_entry_statuses(&normalized)
}

#[tauri::command]
fn set_entry_status(
    archive_path: String,
    entry_path: String,
    status_id: Option<String>,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> AppResult<()> {
    let normalized = normalize_archive_path(&archive_path)?;
    let from = state.db.get_entry_status(&normalized, &entry_path)?;
    state
        .db
        .set_entry_status(&normalized, &entry_path, status_id.as_deref())?;

    if let Ok(archive_id) = state.db.get_archive_id(&normalized) {
        if status_id.is_some() || from.is_some() {
            let _ = state.db.log_action(
                &archive_id,
                Some(&normalized),
                "status_change",
                Some(&entry_path),
                from.as_deref(),
                status_id.as_deref(),
                None,
            );
        }
        if let Some(sid) = status_id.as_deref() {
            let _ = state.db.enqueue_sync(&archive_id, &entry_path, sid);
        }
    }
    let _ = app.emit(
        "hehel:status-changed",
        serde_json::json!({
            "archivePath": normalized,
            "entryPath": entry_path,
            "statusId": status_id,
        }),
    );
    Ok(())
}

#[tauri::command]
fn set_entry_status_bulk(
    archive_path: String,
    entry_paths: Vec<String>,
    status_id: Option<String>,
    state: tauri::State<'_, AppState>,
) -> AppResult<()> {
    let normalized = normalize_archive_path(&archive_path)?;
    for entry_path in entry_paths {
        state
            .db
            .set_entry_status(&normalized, &entry_path, status_id.as_deref())?;
        if let (Ok(archive_id), Some(sid)) =
            (state.db.get_archive_id(&normalized), status_id.as_deref())
        {
            let _ = state.db.enqueue_sync(&archive_id, &entry_path, sid);
        }
    }
    Ok(())
}

#[tauri::command]
fn get_recent_archives(state: tauri::State<'_, AppState>) -> AppResult<Vec<String>> {
    state.db.list_recent()
}

#[tauri::command]
fn get_sync_config(state: tauri::State<'_, AppState>) -> AppResult<SyncConfig> {
    state.db.get_sync_config()
}

#[tauri::command]
fn save_sync_config(config: SyncConfig, state: tauri::State<'_, AppState>) -> AppResult<()> {
    state.db.save_sync_config(&config)
}

#[tauri::command]
async fn sync_with_hestia(state: tauri::State<'_, AppState>) -> AppResult<u32> {
    let (mut config, pending): (SyncConfig, Vec<SyncQueueItem>) = {
        (state.db.get_sync_config()?, state.db.list_pending_sync()?)
    };
    config = HestiaSyncClient::fill_config_from_session(config)?;

    if !config.enabled || config.api_base_url.is_empty() || config.project_id.is_empty() {
        return Err(AppError::Sync(
            "Синхронизация не настроена. Войдите и укажите projectId.".into(),
        ));
    }

    if pending.is_empty() {
        return Ok(0);
    }

    let client = HestiaSyncClient::new();
    let _ = client.init_board(&config).await;

    let mut by_archive: std::collections::HashMap<String, Vec<RemoteArchiveLinkPayload>> =
        std::collections::HashMap::new();

    {
        for item in &pending {
            let remote_status = state.db.resolve_remote_status_id(&item.status_id)?;
            by_archive
                .entry(item.archive_id.clone())
                .or_default()
                .push(RemoteArchiveLinkPayload {
                    entry_path: item.entry_path.clone(),
                    workflow_status_id: remote_status,
                });
        }
    }

    let mut synced = 0u32;
    for (archive_id, links) in by_archive {
        client
            .push_bulk(&config, archive_id, links)
            .await?;
        synced += 1;
    }

    state.db.mark_synced(&pending)?;
    Ok(synced)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExtractSessionResult {
    session_id: String,
    paths: Vec<String>,
}

#[tauri::command]
async fn extract_to_session(
    archive_path: String,
    entries: Vec<String>,
    preserve_paths: bool,
    cache_dir: Option<String>,
    state: tauri::State<'_, AppState>,
) -> AppResult<ExtractSessionResult> {
    let normalized = normalize_archive_path(&archive_path)?;
    let normalized_for_db = normalized.clone();
    let archive_hash = extract_cache::archive_hash(&normalized)?;

    let (session_id, dir) = {
        let mut sessions = state
            .sessions
            .write()
            .map_err(|e| AppError::Archive(e.to_string()))?;
        prune_idle_sessions(&mut sessions);
        if let Some(info) = sessions.get(&archive_hash) {
            let sid = info.session_id.clone();
            let path = temp_session::session_path(&sid);
            if path.is_dir() {
                sessions.insert(
                    archive_hash.clone(),
                    SessionInfo {
                        session_id: sid.clone(),
                        archive_hash: archive_hash.clone(),
                        last_access: Instant::now(),
                    },
                );
                (sid, path)
            } else {
                sessions.remove(&archive_hash);
                let (sid, path) = create_session_dir()?;
                sessions.insert(
                    archive_hash.clone(),
                    SessionInfo {
                        session_id: sid.clone(),
                        archive_hash,
                        last_access: Instant::now(),
                    },
                );
                (sid, path)
            }
        } else {
            let (sid, path) = create_session_dir()?;
            sessions.insert(
                archive_hash.clone(),
                SessionInfo {
                    session_id: sid.clone(),
                    archive_hash,
                    last_access: Instant::now(),
                },
            );
            (sid, path)
        }
    };

    let options = ExtractOptions {
        preserve_paths,
        overwrite: OverwriteMode::Replace,
        extensions_filter: None,
    };
    let service = state.archive_service.clone();
    let cache_dir_for_blocking = cache_dir.clone();
    let paths = tauri::async_runtime::spawn_blocking(move || {
        service.extract_with_cache(
            &normalized,
            &dir.to_string_lossy(),
            &entries,
            &options,
            cache_dir_for_blocking.as_deref(),
        )
    })
    .await
    .map_err(|e| AppError::Archive(e.to_string()))??;
    if let Ok(archive_id) = state.db.get_archive_id(&normalized_for_db) {
        let detail = serde_json::json!({ "count": paths.len() }).to_string();
        let _ = state.db.log_action(
            &archive_id,
            Some(&normalized_for_db),
            "extract",
            None,
            None,
            None,
            Some(&detail),
        );
    }
    Ok(ExtractSessionResult { session_id, paths })
}

#[tauri::command]
fn drop_extract_session(session_id: String, state: tauri::State<'_, AppState>) {
    if let Ok(mut sessions) = state.sessions.write() {
        sessions.retain(|_, info| info.session_id != session_id);
    }
    drop_session(&session_id);
}

#[tauri::command]
async fn warm_extract_cache(
    archive_path: String,
    entries: Vec<String>,
    preserve_paths: bool,
    cache_dir: Option<String>,
    state: tauri::State<'_, AppState>,
) -> AppResult<()> {
    let normalized = normalize_archive_path(&archive_path)?;
    let service = state.archive_service.clone();
    tauri::async_runtime::spawn_blocking(move || {
        service.warm_cache(
            &normalized,
            &entries,
            preserve_paths,
            cache_dir.as_deref(),
        )
    })
    .await
    .map_err(|e| AppError::Archive(e.to_string()))??;
    Ok(())
}

#[tauri::command]
fn copy_files_to_clipboard(
    paths: Vec<String>,
    session_id: Option<String>,
) -> AppResult<()> {
    clipboard::copy_files_to_clipboard(&paths)?;
    if let Some(sid) = session_id {
        temp_session::schedule_drop_session(
            sid,
            temp_session::DEFAULT_CLIPBOARD_SESSION_TTL_SECS,
        );
    }
    Ok(())
}

#[tauri::command]
fn read_clipboard_files() -> AppResult<Vec<String>> {
    clipboard::read_clipboard_files()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateHeheResult {
    archive_id: String,
    output_path: String,
    entry_count: u32,
    total_bytes: u64,
}

fn finish_create(
    db: &WorkflowDb,
    output_path: &str,
    created: HeheCreateResult,
) -> AppResult<CreateHeheResult> {
    let normalized = normalize_archive_path(output_path)?;
    db.register_archive(&normalized, &created.archive_id, None)?;
    let _ = db.log_action(
        &created.archive_id,
        Some(&normalized),
        "create",
        None,
        None,
        None,
        None,
    );
    Ok(CreateHeheResult {
        archive_id: created.archive_id,
        output_path: normalized,
        entry_count: created.entry_count,
        total_bytes: created.total_bytes,
    })
}

#[tauri::command]
async fn create_archive(
    output_path: String,
    file_paths: Vec<String>,
    compression_preset: Option<String>,
    convert_images_to_webp: Option<bool>,
    state: tauri::State<'_, AppState>,
) -> AppResult<CreateHeheResult> {
    let output = ensure_hehe_extension(&output_path);
    let service = state.archive_service.clone();
    let paths = file_paths;
    let preset = compression_preset;
    let convert_webp = convert_images_to_webp;
    let output_for_db = output.clone();
    let created = tauri::async_runtime::spawn_blocking(move || {
        service.create_archive(&output, &paths, preset.as_deref(), convert_webp)
    })
    .await
    .map_err(|e| AppError::Archive(e.to_string()))??;

    finish_create(&state.db, &output_for_db, created)
}

#[tauri::command]
async fn create_hehe_from_archive(
    archive_path: String,
    entry_paths: Vec<String>,
    strip_prefix: Option<String>,
    output_path: String,
    compression_preset: Option<String>,
    convert_images_to_webp: Option<bool>,
    state: tauri::State<'_, AppState>,
) -> AppResult<CreateHeheResult> {
    let output = ensure_hehe_extension(&output_path);
    let normalized_source = normalize_archive_path(&archive_path)?;
    let service = state.archive_service.clone();
    let entries = entry_paths;
    let strip = strip_prefix;
    let preset = compression_preset;
    let convert_webp = convert_images_to_webp;
    let output_for_db = output.clone();
    let created = tauri::async_runtime::spawn_blocking(move || {
        service.create_hehe_from_archive(
            &normalized_source,
            &entries,
            strip.as_deref(),
            &output,
            preset.as_deref(),
            convert_webp,
        )
    })
    .await
    .map_err(|e| AppError::Archive(e.to_string()))??;

    finish_create(&state.db, &output_for_db, created)
}

#[tauri::command]
fn read_hehestl_from_archive(
    archive_path: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<Option<String>> {
    let normalized = normalize_archive_path(&archive_path)?;
    state.archive_service.read_hehestl(&normalized)
}

#[tauri::command]
fn get_action_log(
    archive_id: String,
    limit: u32,
    state: tauri::State<'_, AppState>,
) -> AppResult<Vec<ActionLogEntry>> {
    state.db.get_action_log(&archive_id, limit)
}

const DRAG_PREVIEW_FILE: &str = "drag-out-stack.png";
const DRAG_PREVIEW_MAX_PX: u32 = 48;

fn load_drag_preview_raw(path: &std::path::Path) -> AppResult<Vec<u8>> {
    use image::imageops::FilterType;
    use image::ImageFormat;
    use std::io::Cursor;

    let img = image::open(path)
        .map_err(|e| AppError::Archive(format!("drag preview open: {e}")))?;
    let resized = img.resize_exact(
        DRAG_PREVIEW_MAX_PX,
        DRAG_PREVIEW_MAX_PX,
        FilterType::Triangle,
    );
    let mut buf = Vec::new();
    resized
        .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
        .map_err(|e| AppError::Archive(format!("drag preview encode: {e}")))?;
    Ok(buf)
}

fn resolve_drag_preview_path(resource_dir: Option<&std::path::Path>) -> AppResult<PathBuf> {
    if let Some(dir) = resource_dir {
        let bundled = dir.join(DRAG_PREVIEW_FILE);
        if bundled.is_file() {
            return Ok(bundled);
        }
    }

    let dev_asset = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(DRAG_PREVIEW_FILE);
    if dev_asset.is_file() {
        return dev_asset
            .canonicalize()
            .map_err(|e| AppError::Archive(format!("drag preview path: {e}")));
    }

    let icon_fallback = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("icons")
        .join("32x32.png");
    if icon_fallback.is_file() {
        return Ok(icon_fallback);
    }

    Err(AppError::Archive(format!(
        "Не найден drag preview: {DRAG_PREVIEW_FILE}"
    )))
}

fn drag_preview_path(app: &tauri::AppHandle) -> AppResult<PathBuf> {
    let resource_dir = app.path().resource_dir().ok();
    resolve_drag_preview_path(resource_dir.as_deref())
}

#[tauri::command]
fn start_file_drag(
    app: tauri::AppHandle,
    window_label: String,
    paths: Vec<String>,
    session_id: Option<String>,
) -> AppResult<()> {
    if paths.is_empty() {
        return Err(AppError::Validation("Нет файлов для перетаскивания".into()));
    }
    let window = app
        .get_webview_window(&window_label)
        .ok_or_else(|| AppError::Archive(format!("Окно {window_label} не найдено")))?;

    #[cfg(windows)]
    {
        let path_bufs: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
        let preview_path = drag_preview_path(&app)?;
        let preview_bytes = load_drag_preview_raw(&preview_path)?;
        drag::start_drag(
            &window,
            drag::DragItem::Files(path_bufs),
            drag::Image::Raw(preview_bytes),
            move |_result, _cursor| {
                if let Some(sid) = session_id.as_ref() {
                    drop_session(sid);
                }
            },
            drag::Options {
                mode: drag::DragMode::Copy,
                ..Default::default()
            },
        )
        .map_err(|e| AppError::Archive(format!("drag start: {e}")))?;
    }

    #[cfg(not(windows))]
    {
        let _ = window;
        return Err(AppError::Archive(
            "Drag-out поддерживается только на Windows".into(),
        ));
    }

    Ok(())
}

#[tauri::command]
async fn pull_hestia_statuses(
    archive_path: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<u32> {
    let normalized = normalize_archive_path(&archive_path)?;
    let (mut config, archive_id): (SyncConfig, String) = {
        (
            state.db.get_sync_config()?,
            state.db.get_archive_id(&normalized)?,
        )
    };
    config = HestiaSyncClient::fill_config_from_session(config)?;

    if !config.enabled {
        return Ok(0);
    }

    let client = HestiaSyncClient::new();
    let remote_statuses = client.fetch_workflow_statuses(&config).await?;
    let links = client.pull_links(&config, &archive_id).await?;

    state.db.sync_remote_ids_from_labels(&remote_statuses)?;
    state.db.merge_remote_statuses(&normalized, &links)
}

#[tauri::command]
async fn start_heron_login_cmd(
    heron_auth_url: String,
    hcom_api_url: String,
) -> AppResult<AuthLoginResult> {
    start_heron_login(heron_auth_url, hcom_api_url).await
}

#[tauri::command]
fn get_auth_state() -> AppResult<bool> {
    has_session()
}

#[tauri::command]
fn logout_heron() -> AppResult<()> {
    clear_session()
}

#[tauri::command]
async fn cloud_save_archive(
    archive_path: String,
    label: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<String> {
    let normalized = normalize_archive_path(&archive_path)?;
    let (mut config, archive_id, entries, links) = {
        let archive_id = state.db.get_archive_id(&normalized)?;
        let listing = state.archive_service.list(&normalized)?;
        let manifest_entries: Vec<ManifestEntryDto> = listing
            .into_iter()
            .map(|e| ManifestEntryDto {
                path: e.path,
                size_bytes: e.size,
                is_dir: e.is_dir,
            })
            .collect();
        let status_map = state.db.get_entry_statuses(&normalized)?;
        let mut link_payloads = Vec::new();
        for (entry_path, status_id) in status_map {
            let remote = state.db.resolve_remote_status_id(&status_id)?;
            link_payloads.push(RemoteArchiveLinkPayload {
                entry_path,
                workflow_status_id: remote,
            });
        }
        (
            state.db.get_sync_config()?,
            archive_id,
            manifest_entries,
            link_payloads,
        )
    };
    config = HestiaSyncClient::fill_config_from_session(config)?;
    if config.project_id.is_empty() {
        return Err(AppError::Sync("Укажите projectId в настройках синхронизации".into()));
    }
    let client = HestiaSyncClient::new();
    let _ = client.init_board(&config).await;
    let manifest_hash = client
        .sync_archive(&config, archive_id.clone(), label, entries, links)
        .await?;
    Ok(manifest_hash)
}

fn prune_idle_sessions(sessions: &mut HashMap<String, SessionInfo>) {
    let cutoff = Duration::from_secs(SESSION_IDLE_TTL_SECS);
    let stale: Vec<String> = sessions
        .iter()
        .filter(|(_, info)| info.last_access.elapsed() > cutoff)
        .map(|(k, _)| k.clone())
        .collect();
    for key in stale {
        if let Some(info) = sessions.remove(&key) {
            drop_session(&info.session_id);
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let archive_service = Arc::new(ArchiveService::new().expect("archive service"));
    let preview_cache = PreviewCache::new();
    let _ = temp_session::cleanup_stale_sessions(24);
    let db = WorkflowDb::open().expect("database");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            archive_service,
            preview_cache,
            db,
            sessions: RwLock::new(HashMap::new()),
        })
        .register_uri_scheme_protocol("hehe", |ctx, request| {
            use std::borrow::Cow;

            let uri = request.uri().to_string();
            let response = (|| -> AppResult<(Vec<u8>, String)> {
                let (archive_id, entry_path) = archive::preview_uri::parse_preview_uri(&uri)?;
                let _span = tracing::info_span!(
                    "preview_uri",
                    archive_id = %archive_id,
                    entry = %entry_path
                )
                .entered();
                let start = Instant::now();
                let state = ctx.app_handle().state::<AppState>();
                let (bytes, mime, cache_hit) =
                    resolve_preview_bytes(&state, &archive_id, &entry_path)?;
                tracing::info!(
                    elapsed_ms = start.elapsed().as_millis(),
                    cache_hit,
                    "preview_uri"
                );
                Ok((bytes.to_vec(), mime))
            })();

            match response {
                Ok((bytes, mime)) => tauri::http::Response::builder()
                    .status(200)
                    .header("Content-Type", mime)
                    .header("Cache-Control", "max-age=300")
                    .body(Cow::Owned(bytes))
                    .unwrap(),
                Err(_) => tauri::http::Response::builder()
                    .status(404)
                    .body(Cow::Owned(Vec::new()))
                    .unwrap(),
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_archive_entries,
            list_archive_entries_paginated,
            probe_archive,
            extract_archive,
            normalize_path,
            open_archive_session,
            try_restore_archive_statuses,
            get_workflow_statuses,
            create_workflow_status,
            update_workflow_status,
            delete_workflow_status,
            get_entry_statuses,
            set_entry_status,
            set_entry_status_bulk,
            get_recent_archives,
            get_sync_config,
            save_sync_config,
            sync_with_hestia,
            pull_hestia_statuses,
            start_heron_login_cmd,
            get_auth_state,
            logout_heron,
            cloud_save_archive,
            extract_to_session,
            drop_extract_session,
            warm_extract_cache,
            copy_files_to_clipboard,
            read_clipboard_files,
            create_archive,
            create_hehe_from_archive,
            start_file_drag,
            read_hehestl_from_archive,
            read_preview_bytes,
            get_action_log,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod drag_preview_tests {
    use super::resolve_drag_preview_path;

    #[test]
    fn dev_asset_exists_and_is_png() {
        let path = resolve_drag_preview_path(None).expect("drag preview");
        assert!(path.is_file());
        assert_eq!(path.extension().and_then(|s| s.to_str()), Some("png"));
    }
}
