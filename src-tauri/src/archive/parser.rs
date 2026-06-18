use super::seven_zip::ArchiveEntryDto;

fn build_entry(
    path: String,
    size: u64,
    packed: u64,
    modified: Option<String>,
    is_dir: bool,
) -> ArchiveEntryDto {
    let name = path
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(path.as_str())
        .to_string();
    let extension = name
        .rsplit('.')
        .next()
        .filter(|_| name.contains('.'))
        .unwrap_or("")
        .to_ascii_lowercase();
    ArchiveEntryDto {
        path,
        name,
        size,
        packed_size: packed,
        modified,
        is_dir,
        extension,
    }
}

/// Парсер `7z l -ba` — быстрее `-slt` для больших архивов.
pub fn parse_ba_listing(stdout: &str) -> Vec<ArchiveEntryDto> {
    let mut entries = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.len() < 19 || line.as_bytes().get(4) != Some(&b'-') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 6 {
            continue;
        }
        let modified = Some(format!("{} {}", parts[0], parts[1]));
        let attr = parts[2];
        let size = parts[3].parse().unwrap_or(0);
        let packed = parts[4].parse().unwrap_or(0);
        let path = parts[5..].join(" ");
        if path.is_empty() {
            continue;
        }
        let is_dir = attr.contains('D') || path.ends_with('/') || path.ends_with('\\');
        entries.push(build_entry(path, size, packed, modified, is_dir));
    }
    entries
}

pub fn parse_slt_listing(stdout: &str) -> Vec<ArchiveEntryDto> {
    let mut entries = Vec::new();
    let mut path = String::new();
    let mut size: u64 = 0;
    let mut packed: u64 = 0;
    let mut modified: Option<String> = None;
    let mut is_dir = false;

    let flush = |entries: &mut Vec<ArchiveEntryDto>,
                 path: &mut String,
                 size: &mut u64,
                 packed: &mut u64,
                 modified: &mut Option<String>,
                 is_dir: &mut bool| {
        if path.is_empty() {
            return;
        }
        entries.push(build_entry(
            path.clone(),
            *size,
            *packed,
            modified.take(),
            *is_dir,
        ));
        path.clear();
        *size = 0;
        *packed = 0;
        *is_dir = false;
    };

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            flush(
                &mut entries,
                &mut path,
                &mut size,
                &mut packed,
                &mut modified,
                &mut is_dir,
            );
            continue;
        }
        if let Some(rest) = line.strip_prefix("Path = ") {
            flush(
                &mut entries,
                &mut path,
                &mut size,
                &mut packed,
                &mut modified,
                &mut is_dir,
            );
            path = rest.to_string();
        } else if let Some(rest) = line.strip_prefix("Size = ") {
            size = rest.parse().unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("Packed Size = ") {
            packed = rest.parse().unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("Modified = ") {
            modified = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("Attributes = ") {
            if rest.contains('D') {
                is_dir = true;
            }
        }
    }

    flush(
        &mut entries,
        &mut path,
        &mut size,
        &mut packed,
        &mut modified,
        &mut is_dir,
    );

    entries
        .into_iter()
        .filter(|e| !e.path.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ba_extracts_file_and_folder() {
        let sample = r#"
   Date      Time    Attr         Size   Compressed  Name
-------------------
2025-04-01 12:00:00 D....            0            0  folder
2025-04-01 12:00:00 ....A    100000000     50000000  folder/part.stl
-------------------
"#;
        let entries = parse_ba_listing(sample);
        assert_eq!(entries.len(), 2);
        assert!(entries[0].is_dir);
        assert_eq!(entries[1].name, "part.stl");
        assert_eq!(entries[1].size, 100_000_000);
    }

    #[test]
    fn parse_slt_extracts_file_and_folder() {
        let sample = r#"
Listing archive: test.zip

Path = folder/
Attributes = D
Size = 
Packed Size = 
Modified = 2025-04-01 12:00:00

Path = folder/part.stl
Size = 100000000
Packed Size = 50000000
Modified = 2025-04-01 12:00:00
"#;
        let entries = parse_slt_listing(sample);
        assert_eq!(entries.len(), 2);
        assert!(entries[0].is_dir);
        assert_eq!(entries[1].name, "part.stl");
        assert_eq!(entries[1].size, 100_000_000);
    }
}
