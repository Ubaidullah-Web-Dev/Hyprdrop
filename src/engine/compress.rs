use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result, anyhow};
use crate::engine::image_ops::load_image;
use crate::engine::media_ops::compress_video;
use crate::engine::naming::get_suffixed_output_path;
use crate::engine::pdf_ops::compress_pdf;

#[derive(Debug, Clone)]
pub struct CompressionResult {
    pub output_path: PathBuf,
    pub original_bytes: u64,
    pub compressed_bytes: u64,
    pub percent_saved: f64,
}

/// Compresses image, PDF, or video file, saving to `<Stem> Compressed.<ext>`.
pub fn compress_file(source_path: &Path) -> Result<CompressionResult> {
    let metadata = fs::metadata(source_path)
        .with_context(|| format!("Failed to read metadata for {}", source_path.display()))?;
    let original_bytes = metadata.len();

    let ext = source_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let output_path = get_suffixed_output_path(source_path, "Compressed", None);

    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "bmp" | "tiff" | "tif" => {
            let img = load_image(source_path)?;
            // If image is huge, maintain max 2560px for perceptual compression
            let max_dim = 2560;
            let final_img = if img.width() > max_dim || img.height() > max_dim {
                img.resize(max_dim, max_dim, image::imageops::FilterType::Lanczos3)
            } else {
                img
            };

            // Save with target format
            if ext == "jpg" || ext == "jpeg" {
                let rgb = final_img.to_rgb8();
                let file = fs::File::create(&output_path)?;
                let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(file, 80);
                encoder.encode_image(&rgb)?;
            } else if ext == "webp" {
                final_img.save_with_format(&output_path, image::ImageFormat::WebP)?;
            } else {
                final_img.save_with_format(&output_path, image::ImageFormat::Png)?;
            }
        }
        "pdf" => {
            compress_pdf(source_path, &output_path)?;
        }
        "mp4" | "mkv" | "mov" | "avi" | "webm" => {
            compress_video(source_path, &output_path)?;
        }
        _ => {
            return Err(anyhow!("Unsupported file type for compression: .{}", ext));
        }
    }

    let compressed_bytes = fs::metadata(&output_path)?.len();
    let percent_saved = if original_bytes > compressed_bytes {
        ((original_bytes - compressed_bytes) as f64 / original_bytes as f64) * 100.0
    } else {
        0.0
    };

    Ok(CompressionResult {
        output_path,
        original_bytes,
        compressed_bytes,
        percent_saved,
    })
}
