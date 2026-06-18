use super::db::WorkflowDb;
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct SidecarPayload {
    archive_id: String,
}

pub fn sidecar_path_for(archive_path: &str) -> PathBuf {
    PathBuf::from(format!("{archive_path}.hehel"))
}

pub fn ensure_sidecar(archive_path: &str, db: &WorkflowDb) -> AppResult<String> {
    if let Ok(id) = db.get_archive_id(archive_path) {
        return Ok(id);
    }

    let sidecar = sidecar_path_for(archive_path);
    let archive_id = if sidecar.is_file() {
        read_sidecar_id(&sidecar)?
    } else {
        let id = Uuid::new_v4().to_string();
        write_sidecar(&sidecar, &id)?;
        hide_file(&sidecar);
        id
    };

    db.register_archive(
        archive_path,
        &archive_id,
        Some(sidecar.to_string_lossy().as_ref()),
    )?;
    Ok(archive_id)
}

pub fn try_restore_statuses(archive_path: &str, db: &WorkflowDb) -> AppResult<Option<u32>> {
    let sidecar = sidecar_path_for(archive_path);
    if !sidecar.is_file() {
        return Ok(None);
    }

    let archive_id = read_sidecar_id(&sidecar)?;
    let Some(old_path) = db.find_path_by_archive_id(&archive_id)? else {
        db.register_archive(
            archive_path,
            &archive_id,
            Some(sidecar.to_string_lossy().as_ref()),
        )?;
        return Ok(None);
    };

    if old_path == archive_path {
        return Ok(None);
    }

    let count = db.relink_archive_path(&old_path, archive_path)?;
    db.register_archive(
        archive_path,
        &archive_id,
        Some(sidecar.to_string_lossy().as_ref()),
    )?;
    Ok(Some(count))
}

fn read_sidecar_id(path: &Path) -> AppResult<String> {
    let raw = fs::read_to_string(path)?;
    let payload: SidecarPayload = serde_json::from_str(&raw)
        .map_err(|e| AppError::Validation(format!("sidecar parse: {e}")))?;
    Ok(payload.archive_id)
}

fn write_sidecar(path: &Path, archive_id: &str) -> AppResult<()> {
    let payload = SidecarPayload {
        archive_id: archive_id.to_string(),
    };
    let json = serde_json::to_string_pretty(&payload)
        .map_err(|e| AppError::Validation(e.to_string()))?;
    fs::write(path, json)?;
    Ok(())
}

fn hide_file(path: &Path) {
    #[cfg(windows)]
    {
        let _ = Command::new("attrib").args(["+h", &path.to_string_lossy()]).output();
    }
}
