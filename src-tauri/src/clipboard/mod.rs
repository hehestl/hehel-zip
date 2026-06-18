use crate::error::{AppError, AppResult};

#[cfg(windows)]
pub fn copy_files_to_clipboard(paths: &[String]) -> AppResult<()> {
    use clipboard_win::{formats, Clipboard, Setter};

    if paths.is_empty() {
        return Err(AppError::Validation("Нет файлов для копирования".into()));
    }

    let _clip = Clipboard::new_attempts(10)
        .map_err(|e| AppError::Archive(format!("clipboard open: {e}")))?;
    formats::FileList
        .write_clipboard(paths)
        .map_err(|e| AppError::Archive(format!("clipboard write: {e}")))?;
    Ok(())
}

#[cfg(windows)]
pub fn read_clipboard_files() -> AppResult<Vec<String>> {
    use clipboard_win::{formats, Clipboard, Getter};

    let _clip = Clipboard::new_attempts(10)
        .map_err(|e| AppError::Archive(format!("clipboard open: {e}")))?;
    let mut paths = Vec::new();
    let count = formats::FileList
        .read_clipboard(&mut paths)
        .map_err(|e| AppError::Archive(format!("clipboard read: {e}")))?;
    if count == 0 {
        return Ok(vec![]);
    }
    Ok(paths)
}

#[cfg(not(windows))]
pub fn copy_files_to_clipboard(_paths: &[String]) -> AppResult<()> {
    Err(AppError::Archive(
        "Копирование файлов в буфер поддерживается только на Windows".into(),
    ))
}

#[cfg(not(windows))]
pub fn read_clipboard_files() -> AppResult<Vec<String>> {
    Ok(vec![])
}

#[cfg(all(windows, test))]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn clipboard_file_list_roundtrip() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("clip-test.txt");
        fs::write(&file, b"test").unwrap();
        let path = file.to_string_lossy().into_owned();
        copy_files_to_clipboard(&[path.clone()]).unwrap();
        let read = read_clipboard_files().unwrap();
        assert!(read.iter().any(|p| p.eq_ignore_ascii_case(&path)));
    }
}
