use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::{Context, Result, anyhow};
use lopdf::{Document, Object};

/// Extracts plain text from all pages in a PDF using `lopdf`.
pub fn extract_pdf_text(pdf_path: &Path) -> Result<String> {
    let doc = Document::load(pdf_path)
        .with_context(|| format!("Failed to parse PDF: {}", pdf_path.display()))?;

    let mut full_text = String::new();
    let pages = doc.get_pages();
    let mut page_numbers: Vec<u32> = pages.keys().copied().collect();
    page_numbers.sort_unstable();

    for page_num in page_numbers {
        if let Ok(text) = doc.extract_text(&[page_num])
            && !text.trim().is_empty() {
                if !full_text.is_empty() {
                    full_text.push_str("\n\n--- Page Break ---\n\n");
                }
                full_text.push_str(text.trim());
            }
    }

    if full_text.trim().is_empty() {
        // Fallback: search for all stream text
        for object in doc.objects.values() {
            if let Object::Stream(stream) = object
                && let Ok(content) = stream.decompressed_content()
                    && let Ok(s) = std::str::from_utf8(&content) {
                        for line in s.lines() {
                            if line.ends_with("Tj") || line.ends_with("TJ") {
                                full_text.push_str(line);
                                full_text.push('\n');
                            }
                        }
                    }
        }
    }

    Ok(full_text)
}

/// Renders PDF page to PNG or JPG using `pdftoppm`.
pub fn render_pdf_to_image(pdf_path: &Path, output_path: &Path, format: &str) -> Result<PathBuf> {
    let parent = output_path.parent().unwrap_or_else(|| Path::new("."));
    let temp_prefix = parent.join(format!("hyprdrop_pdf_render_{}", std::process::id()));

    let fmt_arg = match format.to_lowercase().as_str() {
        "jpg" | "jpeg" => "-jpeg",
        _ => "-png",
    };

    let status = Command::new("pdftoppm")
        .arg(fmt_arg)
        .arg("-r")
        .arg("150")
        .arg("-f")
        .arg("1")
        .arg("-l")
        .arg("1")
        .arg(pdf_path)
        .arg(&temp_prefix)
        .status()
        .with_context(|| "Failed to execute pdftoppm. Is poppler installed?")?;

    if !status.success() {
        return Err(anyhow!("pdftoppm exited with failure status"));
    }

    // Look for generated file (typically prefix-1.png or prefix-01.png or prefix-1.jpg)
    let ext = if fmt_arg == "-jpeg" { "jpg" } else { "png" };
    let prefix_name = temp_prefix
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow!("Invalid temporary prefix filename"))?;
    let candidate1 = parent.join(format!("{}-1.{}", prefix_name, ext));
    let candidate2 = parent.join(format!("{}-01.{}", prefix_name, ext));

    let found = if candidate1.exists() {
        candidate1
    } else if candidate2.exists() {
        candidate2
    } else {
        return Err(anyhow!("Could not locate rendered image from pdftoppm"));
    };

    fs::rename(&found, output_path)
        .with_context(|| format!("Failed to move rendered PDF page to {}", output_path.display()))?;

    Ok(output_path.to_path_buf())
}

/// Converts PDF to TXT, MD, HTML, DOCX, or Image formats.
pub fn convert_pdf_to(pdf_path: &Path, output_path: &Path, target_ext: &str) -> Result<PathBuf> {
    let ext_lower = target_ext.trim_start_matches('.').to_lowercase();
    match ext_lower.as_str() {
        "png" | "jpg" | "jpeg" | "webp" => {
            if ext_lower == "webp" {
                let temp_png = output_path.with_extension("temp.png");
                render_pdf_to_image(pdf_path, &temp_png, "png")?;
                let img = image::open(&temp_png)?;
                img.save_with_format(output_path, image::ImageFormat::WebP)?;
                let _ = fs::remove_file(temp_png);
                Ok(output_path.to_path_buf())
            } else {
                render_pdf_to_image(pdf_path, output_path, &ext_lower)
            }
        }
        "txt" => {
            let text = extract_pdf_text(pdf_path)?;
            fs::write(output_path, text)?;
            Ok(output_path.to_path_buf())
        }
        "md" => {
            let text = extract_pdf_text(pdf_path)?;
            let stem = pdf_path.file_stem().and_then(|s| s.to_str()).unwrap_or("Document");
            let mut md = format!("# {}\n\n", stem);
            for para in text.split("\n\n") {
                if !para.trim().is_empty() {
                    md.push_str(para.trim());
                    md.push_str("\n\n");
                }
            }
            fs::write(output_path, md)?;
            Ok(output_path.to_path_buf())
        }
        "html" => {
            let text = extract_pdf_text(pdf_path)?;
            let stem = pdf_path.file_stem().and_then(|s| s.to_str()).unwrap_or("Document");
            let mut html = String::new();
            html.push_str("<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n");
            html.push_str(&format!("<title>{}</title>\n", stem));
            html.push_str("<style>body{font-family:sans-serif;line-height:1.6;max-width:800px;margin:40px auto;padding:0 20px;background:#161618;color:#f4f4f6;}hr{border:1px solid #333;margin:30px 0;}</style>\n</head>\n<body>\n");
            html.push_str(&format!("<h1>{}</h1>\n", stem));
            for para in text.split("\n\n") {
                if para.contains("--- Page Break ---") {
                    html.push_str("<hr>\n");
                } else if !para.trim().is_empty() {
                    html.push_str(&format!("<p>{}</p>\n", html_escape(para.trim())));
                }
            }
            html.push_str("</body>\n</html>");
            fs::write(output_path, html)?;
            Ok(output_path.to_path_buf())
        }
        "docx" => {
            if let Ok(text) = extract_pdf_text(pdf_path) {
                let temp_txt = output_path.with_extension("temp.txt");
                fs::write(&temp_txt, &text)?;
                let status = Command::new("pandoc")
                    .arg(&temp_txt)
                    .arg("-o")
                    .arg(output_path)
                    .status();

                let _ = fs::remove_file(&temp_txt);
                if status.map(|s| s.success()).unwrap_or(false) {
                    return Ok(output_path.to_path_buf());
                }

                // Fallback: LibreOffice headless conversion
                let out_dir = output_path.parent().unwrap_or_else(|| Path::new("."));
                let lo_cmd = if Command::new("libreoffice").arg("--version").output().is_ok() {
                    "libreoffice"
                } else {
                    "soffice"
                };
                let lo_status = Command::new(lo_cmd)
                    .args(["--headless", "--convert-to", "docx", "--outdir"])
                    .arg(out_dir)
                    .arg(pdf_path)
                    .status();
                if lo_status.map(|s| s.success()).unwrap_or(false) {
                    return Ok(output_path.to_path_buf());
                }
            }
            Err(anyhow!("Pandoc or LibreOffice is required for DOCX compilation"))
        }
        "pdf" => {
            // Optimize/re-compress PDF
            compress_pdf(pdf_path, output_path)?;
            Ok(output_path.to_path_buf())
        }
        other => Err(anyhow!("Unsupported target document format: {}", other)),
    }
}

/// Reads metadata dictionary from PDF.
pub fn inspect_pdf_metadata(pdf_path: &Path) -> Result<HashMap<String, String>> {
    let doc = Document::load(pdf_path)
        .with_context(|| format!("Failed to read PDF: {}", pdf_path.display()))?;

    let mut meta = HashMap::new();
    let maybe_dict = if let Ok(info_ref) = doc.trailer.get(b"Info").and_then(|o| o.as_reference()) {
        doc.get_dictionary(info_ref).ok()
    } else {
        doc.trailer.get(b"Info").and_then(|o| o.as_dict()).ok()
    };

    if let Some(info_dict) = maybe_dict {
        for (k, v) in info_dict.iter() {
            let key_str = String::from_utf8_lossy(k).to_string();
            let val_str = match v {
                Object::String(s, _) => String::from_utf8_lossy(s).to_string(),
                Object::Name(n) => String::from_utf8_lossy(n).to_string(),
                Object::Integer(i) => i.to_string(),
                _ => format!("{:?}", v),
            };
            meta.insert(key_str, val_str);
        }
    }
    meta.insert("PageCount".to_string(), doc.get_pages().len().to_string());
    Ok(meta)
}

/// Strips all author, creator, GPS, and XMP metadata from PDF and saves sanitized version.
pub fn strip_pdf_metadata(pdf_path: &Path, output_path: &Path) -> Result<()> {
    let mut doc = Document::load(pdf_path)
        .with_context(|| format!("Failed to read PDF for stripping: {}", pdf_path.display()))?;

    // Clear trailer Info dictionary
    doc.trailer.remove(b"Info");

    // Clear Catalog Metadata stream
    if let Ok(cat_id) = doc.trailer.get(b"Root").and_then(|r| r.as_reference())
        && let Ok(catalog) = doc.get_object_mut(cat_id).and_then(|o| o.as_dict_mut()) {
            catalog.remove(b"Metadata");
        }

    doc.save(output_path)
        .with_context(|| format!("Failed to save stripped PDF to {}", output_path.display()))?;
    Ok(())
}

/// Compresses PDF document by re-deflating streams and removing unreferenced objects.
pub fn compress_pdf(pdf_path: &Path, output_path: &Path) -> Result<()> {
    let mut doc = Document::load(pdf_path)?;
    doc.compress();
    doc.prune_objects();
    doc.save(output_path)?;
    Ok(())
}

fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
