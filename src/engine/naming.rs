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

    // Separate base name and extension, respecting compound archive extensions like .tar.gz
    let (base_stem, extension) = split_stem_and_ext(file_name);

    // Strip existing trailing counter like " (1)" if present to avoid "foo (1) (2).png",
    // while preserving years like " (2024)"
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

/// Helper to split file name into stem and extension (supporting compound extensions).
pub fn split_stem_and_ext(file_name: &str) -> (&str, Option<&str>) {
    for compound in &[".tar.gz", ".tar.zst", ".tar.bz2", ".tar.xz"] {
        if let Some(stem) = file_name.strip_suffix(compound) {
            let ext = &compound[1..];
            return (stem, Some(ext));
        }
    }
    if let Some(dot_idx) = file_name.rfind('.')
        && dot_idx > 0 && dot_idx < file_name.len() - 1 {
            return (&file_name[..dot_idx], Some(&file_name[dot_idx + 1..]));
        }
    (file_name, None)
}

/// Generates a path in the same directory as `source_file` with the specified new extension.
/// Automatically resolves collisions.
pub fn get_converted_output_path(source_file: &Path, new_extension: &str) -> PathBuf {
    let parent = source_file.parent().unwrap_or_else(|| Path::new("."));
    let file_name = source_file.file_name().and_then(|n| n.to_str()).unwrap_or("converted");
    let (stem, _) = split_stem_and_ext(file_name);

    let clean_ext = new_extension.trim_start_matches('.');
    let candidate = parent.join(format!("{}.{}", stem, clean_ext));
    get_collision_safe_path(&candidate)
}

/// Generates an edited output path with a custom suffix (e.g. "Cropped", "With Background", "Redacted").
/// Automatically resolves collisions.
pub fn get_suffixed_output_path(source_file: &Path, suffix: &str, new_extension: Option<&str>) -> PathBuf {
    let parent = source_file.parent().unwrap_or_else(|| Path::new("."));
    let file_name = source_file.file_name().and_then(|n| n.to_str()).unwrap_or("output");
    let (stem, orig_ext) = split_stem_and_ext(file_name);

    let ext = match new_extension {
        Some(e) => e.trim_start_matches('.'),
        None => orig_ext.unwrap_or("png"),
    };

    let candidate = parent.join(format!("{} {}.{}", stem, suffix, ext));
    get_collision_safe_path(&candidate)
}

/// Helper to strip trailing " (n)" counter from stem if present.
/// Preserves 4-digit years like (1900..=2099) so `Report (2024).pdf` does not become `Report (2).pdf`.
pub fn strip_existing_counter(stem: &str) -> &str {
    if let Some(open_idx) = stem.rfind(" (")
        && stem.ends_with(')') {
            let inner = &stem[open_idx + 2..stem.len() - 1];
            if let Ok(val) = inner.parse::<u32>()
                && val > 0 && !(1900..=2099).contains(&val) {
                    return &stem[..open_idx];
                }
        }
    stem
}
