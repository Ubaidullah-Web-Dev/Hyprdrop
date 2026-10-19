use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use anyhow::{Context, Result, anyhow};

/// Extracts text from an image using `tesseract` and copies it to clipboard.
/// Returns the extracted text.
pub fn ocr_image_and_copy(image_path: &Path) -> Result<String> {
    let output = Command::new("tesseract")
        .arg(image_path)
        .arg("stdout")
        .arg("-l")
        .arg("eng")
        .output()
        .with_context(|| "Failed to execute tesseract. Is tesseract installed?")?;

    if !output.status.success() {
        return Err(anyhow!(
            "tesseract OCR failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        return Err(anyhow!("No text detected in image"));
    }

    copy_text_to_clipboard(&text)?;
    Ok(text)
}

/// Copies UTF-8 text to Wayland clipboard via `wl-copy`.
pub fn copy_text_to_clipboard(text: &str) -> Result<()> {
    let mut child = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .spawn()
        .with_context(|| "Failed to run wl-copy. Is wl-clipboard installed?")?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())?;
    }

    let status = child.wait()?;
    if !status.success() {
        return Err(anyhow!("wl-copy exited with error"));
    }
    Ok(())
}

/// Copies an image file to Wayland clipboard with MIME type `image/png`.
pub fn copy_image_to_clipboard(image_path: &Path) -> Result<()> {
    let data = std::fs::read(image_path)?;
    let mut child = Command::new("wl-copy")
        .arg("--type")
        .arg("image/png")
        .stdin(Stdio::piped())
        .spawn()
        .with_context(|| "Failed to run wl-copy. Is wl-clipboard installed?")?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(&data)?;
    }

    let status = child.wait()?;
    if !status.success() {
        return Err(anyhow!("wl-copy exited with error"));
    }
    Ok(())
}
