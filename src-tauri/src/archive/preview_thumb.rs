use crate::error::{AppError, AppResult};
use image::imageops::FilterType;
use image::ImageReader;

const THUMB_MAX: u32 = 256;

pub fn is_raster_image(mime: &str) -> bool {
    matches!(
        mime,
        "image/png" | "image/jpeg" | "image/jpg" | "image/gif" | "image/webp" | "image/bmp"
    )
}

pub fn to_webp_thumb(bytes: &[u8], mime: &str) -> AppResult<Option<(Vec<u8>, String)>> {
    if !is_raster_image(mime) {
        return Ok(None);
    }

    let reader = ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| AppError::Archive(e.to_string()))?;
    let img = reader
        .decode()
        .map_err(|e| AppError::Archive(e.to_string()))?;

    let (w, h) = (img.width(), img.height());
    let scale = (THUMB_MAX as f32 / w.max(h) as f32).min(1.0);
    let nw = ((w as f32) * scale).round().max(1.0) as u32;
    let nh = ((h as f32) * scale).round().max(1.0) as u32;
    let resized = img.resize(nw, nh, FilterType::Triangle);

    let mut out = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut out);
    resized
        .write_to(&mut cursor, image::ImageFormat::WebP)
        .map_err(|e| AppError::Archive(e.to_string()))?;

    Ok(Some((out, "image/webp".into())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_non_image() {
        assert!(to_webp_thumb(b"not image", "text/plain").unwrap().is_none());
    }
}
