use crate::error::{AppError, AppResult};
use image::ImageReader;
use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;

pub fn is_jpg_or_png_path(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| matches!(e.to_ascii_lowercase().as_str(), "jpg" | "jpeg" | "png"))
        .unwrap_or(false)
}

pub fn webp_path_for(path: &str) -> String {
    let p = Path::new(path);
    let stem = p
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(path);
    let file_name = format!("{stem}.webp");
    match p.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        Some(parent) => {
            let parent_str = parent.to_string_lossy();
            format!("{parent_str}/{file_name}").replace('\\', "/")
        }
        None => file_name,
    }
}

pub fn encode_webp_full(bytes: &[u8]) -> AppResult<Vec<u8>> {
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| AppError::Archive(e.to_string()))?;
    let img = reader
        .decode()
        .map_err(|e| AppError::Archive(e.to_string()))?;
    let mut out = Vec::new();
    img.write_to(&mut Cursor::new(&mut out), image::ImageFormat::WebP)
        .map_err(|e| AppError::Archive(e.to_string()))?;
    Ok(out)
}

/// Converts JPG/PNG entries when WebP is smaller. Returns old_path → final_path.
pub fn apply_webp_conversion(
    files: &mut HashMap<String, Vec<u8>>,
) -> AppResult<HashMap<String, String>> {
    let mut path_map = HashMap::new();
    let keys: Vec<String> = files.keys().cloned().collect();

    for key in keys {
        path_map.insert(key.clone(), key.clone());
        if !is_jpg_or_png_path(&key) {
            continue;
        }
        let Some(raw) = files.remove(&key) else {
            continue;
        };
        let webp = match encode_webp_full(&raw) {
            Ok(w) => w,
            Err(_) => {
                files.insert(key.clone(), raw);
                continue;
            }
        };
        if webp.len() >= raw.len() {
            files.insert(key.clone(), raw);
            continue;
        }
        let new_path = webp_path_for(&key);
        if files.contains_key(&new_path) && new_path != key {
            files.insert(key.clone(), raw);
            continue;
        }
        path_map.insert(key.clone(), new_path.clone());
        files.insert(new_path, webp);
    }

    Ok(path_map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};

    #[test]
    fn webp_path_replaces_extension() {
        assert_eq!(webp_path_for("refs/photo.jpg"), "refs/photo.webp");
        assert_eq!(webp_path_for("shot.PNG"), "shot.webp");
    }

    #[test]
    fn converts_png_when_smaller() {
        let mut img = ImageBuffer::new(64, 64);
        for p in img.pixels_mut() {
            *p = Rgb([120, 40, 200]);
        }
        let mut png = Vec::new();
        img.write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();

        let mut files = HashMap::from([("ref.png".to_string(), png.clone())]);
        let map = apply_webp_conversion(&mut files).unwrap();
        assert_eq!(map.get("ref.png"), Some(&"ref.webp".to_string()));
        assert!(files.contains_key("ref.webp"));
        assert!(!files.contains_key("ref.png"));
        assert!(files["ref.webp"].len() < png.len());
    }
}
