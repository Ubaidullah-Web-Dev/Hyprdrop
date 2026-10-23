pub mod archive_ops;
pub mod compress;
pub mod image_ops;
pub mod media_ops;
pub mod metadata;
pub mod naming;
pub mod ocr_clipboard;
pub mod pdf_ops;
pub mod perimeter_math;
pub mod redact_annotate;

/// Compacts and returns unused heap memory back to the operating system kernel.
pub fn trim_memory() {
    #[cfg(target_os = "linux")]
    unsafe {
        libc::malloc_trim(0);
    }
}

use std::path::{Path, PathBuf};
use anyhow::{Result, anyhow};
use crate::engine::archive_ops::{create_tar_gz, create_tar_zst, create_zip, extract_archive};
use crate::engine::image_ops::convert_image_format;
use crate::engine::media_ops::convert_media;
use crate::engine::naming::get_converted_output_path;
use crate::engine::pdf_ops::convert_pdf_to;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileCategory {
    DocumentPdf,
    Image,
    AudioVideo,
    Archive,
    Other,
}

impl FileCategory {
    pub fn from_path(path: &Path) -> Self {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "pdf" | "docx" | "doc" | "txt" | "md" | "html" | "rtf" | "odt" => Self::DocumentPdf,
            "png" | "jpg" | "jpeg" | "webp" | "avif" | "tiff" | "tif" | "bmp" | "ico" | "svg" | "heic" => Self::Image,
            "mp4" | "mkv" | "mov" | "avi" | "webm" | "gif" | "mp3" | "wav" | "flac" | "opus" | "ogg" | "m4a" => Self::AudioVideo,
            "zip" | "tar" | "gz" | "zst" | "tgz" | "tzst" | "bz2" | "xz" | "7z" | "rar" => Self::Archive,
            _ => {
                // If filename has multi-extension like .tar.gz
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
                if name.ends_with(".tar.gz") || name.ends_with(".tar.zst") || name.ends_with(".tar.xz") || name.ends_with(".tar.bz2") {
                    Self::Archive
                } else {
                    Self::Other
                }
            }
        }
    }
}

/// Returns dynamic format slices for Mode 1 based on file category.
pub fn get_conversion_slices(category: FileCategory) -> &'static [&'static str] {
    match category {
        FileCategory::DocumentPdf => &["DOCX", "JPG", "PNG", "TXT", "MD", "HTML", "PDF"],
        FileCategory::Image => &["PNG", "JPG", "WEBP", "AVIF", "TIFF", "BMP", "ICO", "PDF"],
        FileCategory::AudioVideo => &["MP4", "WEBM", "GIF", "MP3", "WAV", "FLAC", "OPUS"],
        FileCategory::Archive => &["ZIP", "TAR.GZ", "TAR.ZST", "TAR.XZ", "7Z", "TAR"],
        FileCategory::Other => &["ZIP", "TAR.ZST", "TAR.GZ", "HEX VIEW"],
    }
}

/// Returns quick-action slices for Mode 2.
pub fn get_quick_action_slices(category: FileCategory) -> &'static [&'static str] {
    match category {
        FileCategory::Archive => &[
            "EXTRACT HERE",
            "EXTRACT TO...",
            "LIST FILES",
            "TEST ARCHIVE",
            "COMPRESS",
            "METADATA",
        ],
        _ => &[
            "CROP",
            "ADD BG",
            "COMPRESS",
            "REDACT",
            "AUTO TRIM",
            "METADATA",
            "EDIT",
            "OCR / COPY",
        ],
    }
}

/// Formats a byte size into clean human-readable text (e.g. `410 KB`, `1.2 MB`).
pub fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Executes format conversion on a file in background/thread.
pub fn execute_conversion(source_path: &Path, target_slice: &str) -> Result<PathBuf> {
    let category = FileCategory::from_path(source_path);

    match category {
        FileCategory::Image => {
            let output_path = get_converted_output_path(source_path, target_slice);
            convert_image_format(source_path, &output_path, target_slice)?;
            Ok(output_path)
        }
        FileCategory::DocumentPdf => {
            let output_path = get_converted_output_path(source_path, target_slice);
            convert_pdf_to(source_path, &output_path, target_slice)
        }
        FileCategory::AudioVideo => {
            let output_path = get_converted_output_path(source_path, target_slice);
            convert_media(source_path, &output_path, target_slice)?;
            Ok(output_path)
        }
        FileCategory::Archive => {
            let parent = source_path.parent().unwrap_or_else(|| Path::new("."));
            let target_upper = target_slice.to_uppercase();
            match target_upper.as_str() {
                "EXTRACT HERE" => {
                    let stem = source_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("extracted");
                    let dest = parent.join(stem);
                    std::fs::create_dir_all(&dest)?;
                    extract_archive(source_path, &dest)?;
                    Ok(dest)
                }
                "ZIP" | "TAR.GZ" | "TAR.ZST" | "TAR.XZ" | "7Z" | "TAR" | "TGZ" | "TZST" | "TXZ" => {
                    let ext = target_slice.to_lowercase();
                    let out = get_converted_output_path(source_path, &ext);
                    crate::engine::archive_ops::convert_archive_format(source_path, &target_upper, &out)?;
                    Ok(out)
                }
                other => Err(anyhow!("Unknown archive slice action: {}", other)),
            }
        }
        FileCategory::Other => {
            let target_upper = target_slice.to_uppercase();
            match target_upper.as_str() {
                "ZIP" => {
                    let out = get_converted_output_path(source_path, "zip");
                    create_zip(source_path, &out)?;
                    Ok(out)
                }
                "TAR.ZST" | "TZST" => {
                    let out = get_converted_output_path(source_path, "tar.zst");
                    create_tar_zst(source_path, &out)?;
                    Ok(out)
                }
                "TAR.GZ" | "TGZ" => {
                    let out = get_converted_output_path(source_path, "tar.gz");
                    create_tar_gz(source_path, &out)?;
                    Ok(out)
                }
                _ => Err(anyhow!("Unsupported conversion for this file type")),
            }
        }
    }
}

/// Executes format conversion on multiple files concurrently using Rayon.
pub fn execute_conversion_batch(source_paths: &[PathBuf], target_slice: &str) -> Vec<Result<PathBuf>> {
    use rayon::prelude::*;
    source_paths
        .par_iter()
        .map(|path| execute_conversion(path, target_slice))
        .collect()
}
