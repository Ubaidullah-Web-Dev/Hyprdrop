use std::path::{Path, PathBuf};

/// Generates a collision-safe file path.
/// If `target_path` does not exist, returns `target_path`.
/// If it exists, appends ` (2)`, ` (3)`, etc. before the file extension.
/// Handles Unicode characters, spaces, dotted names, and files without extensions.
pub fn get_collision_safe_path(target_path: &Path) -> PathBuf {
    if !target_path.exists() {
        return target_path.to_path_buf();
    }

    let parent = target_path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = match target_path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name,
        None => return target_path.to_path_buf(),
    };

    // Separate base name and extension
    let (base_stem, extension) = match target_path.extension().and_then(|e| e.to_str()) {
        Some(ext) => {
            let stem = &file_name[..file_name.len() - ext.len() - 1];
            (stem, Some(ext))
        }
        None => (file_name, None),
    };

    // Strip existing trailing counter like " (1)" if present to avoid "foo (1) (2).png"
    let clean_stem = strip_existing_counter(base_stem);

    let mut counter = 2;
    loop {
        let new_file_name = match extension {
            Some(ext) => format!("{} ({}).{}", clean_stem, counter, ext),
            None => format!("{} ({})", clean_stem, counter),
        };

        let candidate = parent.join(new_file_name);
        if !candidate.exists() {
            return candidate;
        }
        counter += 1;
    }
}

/// Generates a path in the same directory as `source_file` with the specified new extension.
/// Automatically resolves collisions.
pub fn get_converted_output_path(source_file: &Path, new_extension: &str) -> PathBuf {
    let parent = source_file.parent().unwrap_or_else(|| Path::new("."));
    let stem = source_file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("converted");

    let clean_ext = new_extension.trim_start_matches('.');
    let candidate = parent.join(format!("{}.{}", stem, clean_ext));
    get_collision_safe_path(&candidate)
}

/// Generates an edited output path with a custom suffix (e.g. "Cropped", "With Background", "Redacted").
/// Automatically resolves collisions.
pub fn get_suffixed_output_path(source_file: &Path, suffix: &str, new_extension: Option<&str>) -> PathBuf {
    let parent = source_file.parent().unwrap_or_else(|| Path::new("."));
    let stem = source_file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    let ext = match new_extension {
        Some(e) => e.trim_start_matches('.'),
        None => source_file
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png"),
    };

    let candidate = parent.join(format!("{} {}.{}", stem, suffix, ext));
    get_collision_safe_path(&candidate)
}

/// Helper to strip trailing " (n)" counter from stem if present.
fn strip_existing_counter(stem: &str) -> &str {
    if let Some(open_idx) = stem.rfind(" (") {
        if stem.ends_with(')') {
            let inner = &stem[open_idx + 2..stem.len() - 1];
            if inner.chars().all(|c| c.is_ascii_digit()) && !inner.is_empty() {
                return &stem[..open_idx];
            }
        }
    }
    stem
}
