use crate::error::{AppError, AppResult};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

pub fn sessions_root() -> PathBuf {
    std::env::temp_dir().join("Hehel-Zip")
}

pub fn create_session_dir() -> AppResult<(String, PathBuf)> {
    let id = Uuid::new_v4().to_string();
    let path = sessions_root().join(format!("session-{id}"));
    fs::create_dir_all(&path).map_err(|e| AppError::Archive(format!("session dir: {e}")))?;
    Ok((id, path))
}

pub fn session_path(session_id: &str) -> PathBuf {
    sessions_root().join(format!("session-{session_id}"))
}

pub fn drop_session(session_id: &str) {
    let path = session_path(session_id);
    let _ = fs::remove_dir_all(path);
}

pub const DEFAULT_CLIPBOARD_SESSION_TTL_SECS: u64 = 600;

pub fn schedule_drop_session(session_id: String, delay_secs: u64) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(delay_secs));
        drop_session(&session_id);
    });
}

pub fn cleanup_stale_sessions(max_age_hours: u64) -> AppResult<()> {
    let root = sessions_root();
    if !root.is_dir() {
        return Ok(());
    }

    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(max_age_hours * 3600))
        .unwrap_or(SystemTime::UNIX_EPOCH);

    for entry in fs::read_dir(&root).map_err(|e| AppError::Archive(format!("read sessions: {e}")))? {
        let entry = entry.map_err(|e| AppError::Archive(format!("session entry: {e}")))?;
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if !name.starts_with("session-") {
            continue;
        }
        let modified = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        if modified < cutoff {
            let _ = fs::remove_dir_all(&path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_removes_stale_sessions() {
        let (_, path) = create_session_dir().unwrap();
        assert!(path.exists());
        cleanup_stale_sessions(0).unwrap();
        assert!(!path.exists());
    }
}
