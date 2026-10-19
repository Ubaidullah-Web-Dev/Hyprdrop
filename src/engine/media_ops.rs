use std::path::Path;
use std::process::Command;
use anyhow::{Context, Result, anyhow};

/// Converts audio/video files using `ffmpeg`.
pub fn convert_media(source_path: &Path, output_path: &Path, target_ext: &str) -> Result<()> {
    let ext_lower = target_ext.trim_start_matches('.').to_lowercase();

    match ext_lower.as_str() {
        "gif" => {
            // High-quality palettegen 15fps Lanczos conversion
            let filter = "fps=15,scale=flags=lanczos,split[s0][s1];[s0]palettegen=max_colors=128[p];[s1][p]paletteuse=dither=bayer:bayer_scale=3";
            let status = Command::new("ffmpeg")
                .arg("-y")
                .arg("-i")
                .arg(source_path)
                .arg("-vf")
                .arg(filter)
                .arg(output_path)
                .status()
                .with_context(|| "Failed to execute ffmpeg for GIF conversion")?;

            if !status.success() {
                return Err(anyhow!("ffmpeg failed with exit status {:?}", status.code()));
            }
        }
        "mp4" => {
            let status = Command::new("ffmpeg")
                .arg("-y")
                .arg("-i")
                .arg(source_path)
                .arg("-c:v")
                .arg("libx264")
                .arg("-preset")
                .arg("fast")
                .arg("-crf")
                .arg("23")
                .arg("-c:a")
                .arg("aac")
                .arg("-b:a")
                .arg("160k")
                .arg("-movflags")
                .arg("+faststart")
                .arg(output_path)
                .status()
                .with_context(|| "Failed to execute ffmpeg for MP4 conversion")?;

            if !status.success() {
                return Err(anyhow!("ffmpeg failed with exit status {:?}", status.code()));
            }
        }
        "webm" => {
            let status = Command::new("ffmpeg")
                .arg("-y")
                .arg("-i")
                .arg(source_path)
                .arg("-c:v")
                .arg("libvpx-vp9")
                .arg("-crf")
                .arg("30")
                .arg("-b:v")
                .arg("0")
                .arg("-c:a")
                .arg("libopus")
                .arg("-b:a")
                .arg("128k")
                .arg(output_path)
                .status()
                .with_context(|| "Failed to execute ffmpeg for WEBM conversion")?;

            if !status.success() {
                return Err(anyhow!("ffmpeg failed with exit status {:?}", status.code()));
            }
        }
        "mp3" => {
            let status = Command::new("ffmpeg")
                .arg("-y")
                .arg("-i")
                .arg(source_path)
                .arg("-vn")
                .arg("-c:a")
                .arg("libmp3lame")
                .arg("-b:a")
                .arg("192k")
                .arg(output_path)
                .status()
                .with_context(|| "Failed to execute ffmpeg for MP3 extraction")?;

            if !status.success() {
                return Err(anyhow!("ffmpeg failed with exit status {:?}", status.code()));
            }
        }
        "wav" => {
            let status = Command::new("ffmpeg")
                .arg("-y")
                .arg("-i")
                .arg(source_path)
                .arg("-vn")
                .arg("-c:a")
                .arg("pcm_s16le")
                .arg(output_path)
                .status()
                .with_context(|| "Failed to execute ffmpeg for WAV extraction")?;

            if !status.success() {
                return Err(anyhow!("ffmpeg failed with exit status {:?}", status.code()));
            }
        }
        "flac" => {
            let status = Command::new("ffmpeg")
                .arg("-y")
                .arg("-i")
                .arg(source_path)
                .arg("-vn")
                .arg("-c:a")
                .arg("flac")
                .arg(output_path)
                .status()
                .with_context(|| "Failed to execute ffmpeg for FLAC extraction")?;

            if !status.success() {
                return Err(anyhow!("ffmpeg failed with exit status {:?}", status.code()));
            }
        }
        "opus" => {
            let status = Command::new("ffmpeg")
                .arg("-y")
                .arg("-i")
                .arg(source_path)
                .arg("-vn")
                .arg("-c:a")
                .arg("libopus")
                .arg("-b:a")
                .arg("128k")
                .arg(output_path)
                .status()
                .with_context(|| "Failed to execute ffmpeg for OPUS extraction")?;

            if !status.success() {
                return Err(anyhow!("ffmpeg failed with exit status {:?}", status.code()));
            }
        }
        other => {
            return Err(anyhow!("Unsupported target media format: {}", other));
        }
    }

    Ok(())
}

/// Compresses a video file using modern H.264 CRF 28 and AAC compression.
pub fn compress_video(source_path: &Path, output_path: &Path) -> Result<()> {
    let status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(source_path)
        .arg("-c:v")
        .arg("libx264")
        .arg("-crf")
        .arg("28")
        .arg("-preset")
        .arg("slow")
        .arg("-c:a")
        .arg("aac")
        .arg("-b:a")
        .arg("128k")
        .arg("-movflags")
        .arg("+faststart")
        .arg(output_path)
        .status()
        .with_context(|| "Failed to execute ffmpeg for video compression")?;

    if !status.success() {
        return Err(anyhow!("ffmpeg video compression failed"));
    }
    Ok(())
}
