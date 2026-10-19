use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use crate::engine::image_ops::{load_image, save_image_with_format};
use crate::engine::naming::get_suffixed_output_path;
use crate::engine::pdf_ops::{inspect_pdf_metadata, strip_pdf_metadata};

#[derive(Debug, Clone)]
pub struct FileMetadataInfo {
    pub file_path: PathBuf,
    pub file_size_bytes: u64,
    pub fields: BTreeMap<String, String>,
}

/// Inspects metadata of image or PDF file.
pub fn inspect_file_metadata(path: &Path) -> Result<FileMetadataInfo> {
    let file_size_bytes = std::fs::metadata(path)
        .with_context(|| format!("Cannot access {}", path.display()))?
        .len();

    let mut fields = BTreeMap::new();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext == "pdf" {
        if let Ok(pdf_meta) = inspect_pdf_metadata(path) {
            for (k, v) in pdf_meta {
                fields.insert(k, v);
            }
        }
    } else {
        // Try reading EXIF
        if let Ok(file) = File::open(path) {
            let mut bufreader = BufReader::new(file);
            if let Ok(reader) = exif::Reader::new().read_from_container(&mut bufreader) {
                for f in reader.fields() {
                    let tag_name = f.tag.to_string();
                    let val_str = f.display_value().with_unit(&reader).to_string();
                    fields.insert(tag_name, val_str);
                }
            }
        }

        // Try reading dimensions via image
        if let Ok(img) = image::image_dimensions(path) {
            fields.insert("Dimensions".to_string(), format!("{} × {} px", img.0, img.1));
        }
    }

    Ok(FileMetadataInfo {
        file_path: path.to_path_buf(),
        file_size_bytes,
        fields,
    })
}

/// Strips all EXIF, GPS, author, and camera metadata, saving to `<Stem> Sanitized.<ext>`.
pub fn strip_file_metadata(source_path: &Path) -> Result<PathBuf> {
    let ext = source_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let output_path = get_suffixed_output_path(source_path, "Sanitized", None);

    if ext == "pdf" {
        strip_pdf_metadata(source_path, &output_path)?;
    } else {
        // For images, re-encoding raw pixel buffer strips EXIF and ICC profiles completely
        let img = load_image(source_path)?;
        save_image_with_format(&img, &output_path, &ext)?;
    }

    Ok(output_path)
}
