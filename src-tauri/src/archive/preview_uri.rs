use crate::error::{AppError, AppResult};

const MARKER: &str = "preview/";

pub fn parse_preview_uri(uri: &str) -> AppResult<(String, String)> {
    let decoded = urlencoding::decode(uri)
        .map_err(|e| AppError::Archive(e.to_string()))?
        .into_owned();
    let rest = decoded
        .find(MARKER)
        .map(|pos| &decoded[pos + MARKER.len()..])
        .ok_or_else(|| AppError::Archive("bad preview uri".into()))?;
    let rest = rest.split(&['?', '#'][..]).next().unwrap_or(rest);
    let (archive_id, entry_encoded) = rest
        .split_once('/')
        .ok_or_else(|| AppError::Archive("bad preview uri".into()))?;
    let entry_path = urlencoding::decode(entry_encoded)
        .map_err(|e| AppError::Archive(e.to_string()))?
        .into_owned();
    Ok((archive_id.to_string(), entry_path))
}

#[cfg(test)]
mod tests {
    use super::parse_preview_uri;

    #[test]
    fn parses_hehe_scheme() {
        let (id, path) =
            parse_preview_uri("hehe://preview/abc-123/folder%2Fphoto.png").unwrap();
        assert_eq!(id, "abc-123");
        assert_eq!(path, "folder/photo.png");
    }

    #[test]
    fn parses_windows_localhost_scheme() {
        let (id, path) = parse_preview_uri(
            "http://hehe.localhost/preview%2Fabc-123%2Ffolder%252Fphoto.png",
        )
        .unwrap();
        assert_eq!(id, "abc-123");
        assert_eq!(path, "folder/photo.png");
    }
}
