use super::seven_zip::ExtractOptions;
use std::path::Path;

pub fn collect_extracted_paths(
    dest: &Path,
    entries: &[String],
    options: &ExtractOptions,
) -> Vec<String> {
    let mut result = Vec::new();

    if entries.is_empty() {
        collect_files_recursive(dest, &mut result, options);
        return result;
    }

    for entry in entries {
        let target = if options.preserve_paths {
            dest.join(entry.replace('/', "\\"))
        } else {
            dest.join(
                Path::new(entry)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| entry.clone()),
            )
        };
        if target.is_file() && passes_filter(&target, options) {
            result.push(target.to_string_lossy().into_owned());
        } else if target.is_dir() {
            collect_files_recursive(&target, &mut result, options);
        }
    }

    result
}

fn collect_files_recursive(dir: &Path, out: &mut Vec<String>, options: &ExtractOptions) {
    if !dir.is_dir() {
        return;
    }
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(&path, out, options);
        } else if passes_filter(&path, options) {
            out.push(path.to_string_lossy().into_owned());
        }
    }
}

fn passes_filter(path: &Path, options: &ExtractOptions) -> bool {
    let Some(filter) = &options.extensions_filter else {
        return true;
    };
    if filter.is_empty() {
        return true;
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    filter.iter().any(|f| f.to_ascii_lowercase() == ext)
}
