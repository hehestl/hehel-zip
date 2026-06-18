pub mod adapter;
pub mod archive_service;
pub mod backend;
pub mod extract;
pub mod extract_cache;
pub mod hehe_backend;
pub mod hehe_format;
pub mod image_webp;
pub mod parser;
pub mod path_safety;
pub mod preview_cache;
pub mod preview_uri;
pub mod preview_thumb;
pub mod thumb_disk_cache;
pub mod registry;
pub mod zip_handle_cache;
pub mod sevenz_fallback;
pub mod seven_zip;
pub mod temp_session;
pub mod zip_backend;

#[cfg(test)]
mod integration {
    use super::parser::parse_slt_listing;
    use super::seven_zip::{ExtractOptions, SevenZipAdapter};
    use std::io::Write;
    use tempfile::TempDir;

    fn write_fixture_zip(dir: &TempDir) -> std::path::PathBuf {
        let zip_path = dir.path().join("fixture.zip");
        let file = std::fs::File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("part.stl", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"solid test\nendsolid test\n").unwrap();
        zip.finish().unwrap();
        zip_path
    }

    #[test]
    fn list_and_extract_fixture_zip() {
        let dir = TempDir::new().unwrap();
        let zip_path = write_fixture_zip(&dir);
        let adapter = SevenZipAdapter::new().expect("7z");
        let list = adapter
            .list(&zip_path.to_string_lossy())
            .expect("list");
        assert!(!list.is_empty());

        let out = dir.path().join("out");
        let extracted = adapter
            .extract(
                &zip_path.to_string_lossy(),
                &out.to_string_lossy(),
                &[],
                &ExtractOptions::default(),
            )
            .expect("extract");
        assert!(!extracted.is_empty());
    }

    #[test]
    fn parser_handles_multiline_listing() {
        let entries = parse_slt_listing("Path = a.stl\nSize = 10\n\n");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "a.stl");
    }

    #[test]
    fn create_archive_zip() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("part.stl");
        std::fs::write(&source, b"solid x").unwrap();
        let zip_path = dir.path().join("out.zip");
        let adapter = SevenZipAdapter::new().expect("7z");
        adapter
            .create_archive(
                &zip_path.to_string_lossy(),
                &[source.to_string_lossy().into_owned()],
            )
            .expect("create");
        assert!(zip_path.is_file());
        assert!(adapter.probe(&zip_path.to_string_lossy()).unwrap());
    }
}
