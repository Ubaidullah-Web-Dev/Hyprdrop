use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::{Result, anyhow};

#[derive(Debug, Clone)]
pub struct ArchiveEntryInfo {
    pub path: String,
    pub uncompressed_size: u64,
    pub is_dir: bool,
}

/// Extracts archive into its parent directory or a dedicated folder.
pub fn extract_archive(archive_path: &Path, destination_dir: &Path) -> Result<PathBuf> {
    fs::create_dir_all(destination_dir)?;
    let file_name = archive_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let name_lower = file_name.to_lowercase();

    if name_lower.ends_with(".zip") {
        let file = File::open(archive_path)?;
        let mut zip = zip::ZipArchive::new(BufReader::new(file))?;
        for i in 0..zip.len() {
            let mut file = zip.by_index(i)?;
            let outpath = match file.enclosed_name() {
                Some(path) => destination_dir.join(path),
                None => continue,
            };

            if file.is_dir() {
                fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent()
                    && !p.exists() {
                        fs::create_dir_all(p)?;
                    }
                let mut outfile = File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }
        Ok(destination_dir.to_path_buf())
    } else if name_lower.ends_with(".tar.gz") || name_lower.ends_with(".tgz") {
        let file = File::open(archive_path)?;
        let tar = flate2::read::GzDecoder::new(BufReader::new(file));
        let mut archive = tar::Archive::new(tar);
        archive.unpack(destination_dir)?;
        Ok(destination_dir.to_path_buf())
    } else if name_lower.ends_with(".tar.zst") || name_lower.ends_with(".tzst") {
        let file = File::open(archive_path)?;
        let zstd = zstd::stream::Decoder::new(BufReader::new(file))?;
        let mut archive = tar::Archive::new(zstd);
        archive.unpack(destination_dir)?;
        Ok(destination_dir.to_path_buf())
    } else if name_lower.ends_with(".tar") {
        let file = File::open(archive_path)?;
        let mut archive = tar::Archive::new(BufReader::new(file));
        archive.unpack(destination_dir)?;
        Ok(destination_dir.to_path_buf())
    } else if name_lower.ends_with(".tar.xz") || name_lower.ends_with(".txz") || name_lower.ends_with(".xz") {
        let status = Command::new("tar")
            .args(["-xf", &archive_path.to_string_lossy(), "-C", &destination_dir.to_string_lossy()])
            .status()?;
        if status.success() {
            Ok(destination_dir.to_path_buf())
        } else {
            Err(anyhow!("Failed to extract tar.xz archive with system tar"))
        }
    } else if name_lower.ends_with(".7z") {
        let status = Command::new("7z")
            .args(["x", &archive_path.to_string_lossy(), &format!("-o{}", destination_dir.display()), "-y"])
            .status()?;
        if status.success() {
            Ok(destination_dir.to_path_buf())
        } else {
            Err(anyhow!("Failed to extract 7z archive with system 7z"))
        }
    } else {
        // Fallback: try tar -xf or 7z x
        if let Ok(status) = Command::new("tar")
            .args(["-xf", &archive_path.to_string_lossy(), "-C", &destination_dir.to_string_lossy()])
            .status()
            && status.success()
        {
            return Ok(destination_dir.to_path_buf());
        }
        Err(anyhow!("Unsupported archive format: {}", file_name))
    }
}

/// Lists all files contained inside an archive.
pub fn list_archive_entries(archive_path: &Path) -> Result<Vec<ArchiveEntryInfo>> {
    let file_name = archive_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let name_lower = file_name.to_lowercase();
    let mut entries = Vec::new();

    if name_lower.ends_with(".zip") {
        let file = File::open(archive_path)?;
        let mut zip = zip::ZipArchive::new(BufReader::new(file))?;
        for i in 0..zip.len() {
            let file = zip.by_index(i)?;
            entries.push(ArchiveEntryInfo {
                path: file.name().to_string(),
                uncompressed_size: file.size(),
                is_dir: file.is_dir(),
            });
        }
        Ok(entries)
    } else if name_lower.ends_with(".tar.gz") || name_lower.ends_with(".tgz") {
        let file = File::open(archive_path)?;
        let tar = flate2::read::GzDecoder::new(BufReader::new(file));
        let mut archive = tar::Archive::new(tar);
        for entry in archive.entries()? {
            let entry = entry?;
            entries.push(ArchiveEntryInfo {
                path: entry.path()?.to_string_lossy().to_string(),
                uncompressed_size: entry.size(),
                is_dir: entry.header().entry_type().is_dir(),
            });
        }
        Ok(entries)
    } else if name_lower.ends_with(".tar.zst") || name_lower.ends_with(".tzst") {
        let file = File::open(archive_path)?;
        let zstd = zstd::stream::Decoder::new(BufReader::new(file))?;
        let mut archive = tar::Archive::new(zstd);
        for entry in archive.entries()? {
            let entry = entry?;
            entries.push(ArchiveEntryInfo {
                path: entry.path()?.to_string_lossy().to_string(),
                uncompressed_size: entry.size(),
                is_dir: entry.header().entry_type().is_dir(),
            });
        }
        Ok(entries)
    } else if name_lower.ends_with(".tar") {
        let file = File::open(archive_path)?;
        let mut archive = tar::Archive::new(BufReader::new(file));
        for entry in archive.entries()? {
            let entry = entry?;
            entries.push(ArchiveEntryInfo {
                path: entry.path()?.to_string_lossy().to_string(),
                uncompressed_size: entry.size(),
                is_dir: entry.header().entry_type().is_dir(),
            });
        }
        Ok(entries)
    } else {
        // Fallback using tar -tf or 7z l
        if let Ok(output) = Command::new("tar").args(["-tf", &archive_path.to_string_lossy()]).output()
            && output.status.success()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    entries.push(ArchiveEntryInfo {
                        path: trimmed.to_string(),
                        uncompressed_size: 0,
                        is_dir: trimmed.ends_with('/'),
                    });
                }
            }
            return Ok(entries);
        }
        Err(anyhow!("Cannot list entries for archive: {}", file_name))
    }
}

/// Tests archive integrity by decoding all data streams. Returns (entry_count, total_uncompressed_bytes).
pub fn test_archive_integrity(archive_path: &Path) -> Result<(usize, u64)> {
    let file_name = archive_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let name_lower = file_name.to_lowercase();
    let mut total_bytes = 0u64;
    let mut count = 0usize;

    if name_lower.ends_with(".zip") {
        let file = File::open(archive_path)?;
        let mut zip = zip::ZipArchive::new(BufReader::new(file))?;
        for i in 0..zip.len() {
            let mut file = zip.by_index(i)?;
            let bytes = std::io::copy(&mut file, &mut std::io::sink())?;
            total_bytes += bytes;
            count += 1;
        }
        Ok((count, total_bytes))
    } else if name_lower.ends_with(".tar.gz") || name_lower.ends_with(".tgz") {
        let file = File::open(archive_path)?;
        let tar = flate2::read::GzDecoder::new(BufReader::new(file));
        let mut archive = tar::Archive::new(tar);
        for entry in archive.entries()? {
            let mut entry = entry?;
            let bytes = std::io::copy(&mut entry, &mut std::io::sink())?;
            total_bytes += bytes;
            count += 1;
        }
        Ok((count, total_bytes))
    } else if name_lower.ends_with(".tar.zst") || name_lower.ends_with(".tzst") {
        let file = File::open(archive_path)?;
        let zstd = zstd::stream::Decoder::new(BufReader::new(file))?;
        let mut archive = tar::Archive::new(zstd);
        for entry in archive.entries()? {
            let mut entry = entry?;
            let bytes = std::io::copy(&mut entry, &mut std::io::sink())?;
            total_bytes += bytes;
            count += 1;
        }
        Ok((count, total_bytes))
    } else if name_lower.ends_with(".tar") {
        let file = File::open(archive_path)?;
        let mut archive = tar::Archive::new(BufReader::new(file));
        for entry in archive.entries()? {
            let mut entry = entry?;
            let bytes = std::io::copy(&mut entry, &mut std::io::sink())?;
            total_bytes += bytes;
            count += 1;
        }
        Ok((count, total_bytes))
    } else {
        // Fallback test via tar -tf
        let status = Command::new("tar")
            .args(["-tf", &archive_path.to_string_lossy()])
            .output()?;
        if status.status.success() {
            let lines = String::from_utf8_lossy(&status.stdout).lines().count();
            Ok((lines, 0))
        } else {
            Err(anyhow!("Archive integrity check failed"))
        }
    }
}

/// Converts an archive (e.g. .zip) to another format (.tar.gz, .tar.zst, .tar.xz, .7z, .tar).
pub fn convert_archive_format(source_archive: &Path, target_slice: &str, output_path: &Path) -> Result<PathBuf> {
    let temp_dir = tempfile::tempdir()?;
    extract_archive(source_archive, temp_dir.path())?;

    let target_upper = target_slice.to_uppercase();
    match target_upper.as_str() {
        "ZIP" => {
            create_zip_from_dir_contents(temp_dir.path(), output_path)?;
        }
        "TAR.GZ" | "TGZ" => {
            create_tar_gz_from_dir_contents(temp_dir.path(), output_path)?;
        }
        "TAR.ZST" | "TZST" => {
            create_tar_zst_from_dir_contents(temp_dir.path(), output_path)?;
        }
        "TAR" => {
            create_tar_from_dir_contents(temp_dir.path(), output_path)?;
        }
        "TAR.XZ" | "TXZ" => {
            create_tar_xz_from_dir_contents(temp_dir.path(), output_path)?;
        }
        "7Z" => {
            create_7z_from_dir_contents(temp_dir.path(), output_path)?;
        }
        other => return Err(anyhow!("Unsupported archive target format: {}", other)),
    }

    Ok(output_path.to_path_buf())
}

/// Creates a ZIP archive from the contents of a directory.
pub fn create_zip_from_dir_contents(dir: &Path, output_path: &Path) -> Result<()> {
    let file = File::create(output_path)?;
    let mut zip = zip::ZipWriter::new(BufWriter::new(file));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    add_dir_to_zip(&mut zip, dir, dir, options)?;
    zip.finish()?;
    Ok(())
}

/// Creates a ZIP archive containing the source file or folder.
pub fn create_zip(source_path: &Path, output_path: &Path) -> Result<()> {
    let file = File::create(output_path)?;
    let mut zip = zip::ZipWriter::new(BufWriter::new(file));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    if source_path.is_file() {
        let filename = source_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file");
        zip.start_file(filename, options)?;
        let mut f = File::open(source_path)?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;
        zip.write_all(&buffer)?;
    } else if source_path.is_dir() {
        add_dir_to_zip(&mut zip, source_path, source_path, options)?;
    }

    zip.finish()?;
    Ok(())
}

fn add_dir_to_zip<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    base_dir: &Path,
    current_dir: &Path,
    options: zip::write::SimpleFileOptions,
) -> Result<()> {
    for entry in fs::read_dir(current_dir)? {
        let entry = entry?;
        let path = entry.path();
        let relative_path = path
            .strip_prefix(base_dir)
            .map_err(|e| anyhow!("Strip prefix error: {:?}", e))?;
        let name_str = relative_path
            .to_str()
            .ok_or_else(|| anyhow!("Invalid path name"))?;

        if path.is_dir() {
            zip.add_directory(name_str, options)?;
            add_dir_to_zip(zip, base_dir, &path, options)?;
        } else {
            zip.start_file(name_str, options)?;
            let mut f = File::open(&path)?;
            let mut buf = Vec::new();
            f.read_to_end(&mut buf)?;
            zip.write_all(&buf)?;
        }
    }
    Ok(())
}

/// Helper to recursively pack directory contents into a TAR builder.
fn pack_dir_contents_to_tar<W: Write>(dir: &Path, tar: &mut tar::Builder<W>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("entry");
        if path.is_file() {
            let mut f = File::open(&path)?;
            tar.append_file(name, &mut f)?;
        } else if path.is_dir() {
            tar.append_dir_all(name, &path)?;
        }
    }
    Ok(())
}

/// Creates a TAR.GZ archive containing the source.
pub fn create_tar_gz(source_path: &Path, output_path: &Path) -> Result<()> {
    let file = File::create(output_path)?;
    let gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut tar = tar::Builder::new(gz);

    let name = source_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("entry");

    if source_path.is_file() {
        let mut f = File::open(source_path)?;
        tar.append_file(name, &mut f)?;
    } else if source_path.is_dir() {
        tar.append_dir_all(name, source_path)?;
    }

    tar.finish()?;
    Ok(())
}

/// Creates a TAR.GZ archive directly from directory contents.
pub fn create_tar_gz_from_dir_contents(dir: &Path, output_path: &Path) -> Result<()> {
    let file = File::create(output_path)?;
    let gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut tar = tar::Builder::new(gz);
    pack_dir_contents_to_tar(dir, &mut tar)?;
    tar.finish()?;
    Ok(())
}

/// Creates a TAR.ZST archive containing the source.
pub fn create_tar_zst(source_path: &Path, output_path: &Path) -> Result<()> {
    let file = File::create(output_path)?;
    let zstd = zstd::stream::Encoder::new(file, 3)?;
    let mut tar = tar::Builder::new(zstd.auto_finish());

    let name = source_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("entry");

    if source_path.is_file() {
        let mut f = File::open(source_path)?;
        tar.append_file(name, &mut f)?;
    } else if source_path.is_dir() {
        tar.append_dir_all(name, source_path)?;
    }

    tar.finish()?;
    Ok(())
}

/// Creates a TAR.ZST archive directly from directory contents.
pub fn create_tar_zst_from_dir_contents(dir: &Path, output_path: &Path) -> Result<()> {
    let file = File::create(output_path)?;
    let zstd = zstd::stream::Encoder::new(file, 3)?;
    let mut tar = tar::Builder::new(zstd.auto_finish());
    pack_dir_contents_to_tar(dir, &mut tar)?;
    tar.finish()?;
    Ok(())
}

/// Creates an uncompressed TAR archive containing directory contents.
pub fn create_tar_from_dir_contents(dir: &Path, output_path: &Path) -> Result<()> {
    let file = File::create(output_path)?;
    let mut tar = tar::Builder::new(file);
    pack_dir_contents_to_tar(dir, &mut tar)?;
    tar.finish()?;
    Ok(())
}

/// Creates a TAR.XZ archive from directory contents using system tar.
pub fn create_tar_xz_from_dir_contents(dir: &Path, output_path: &Path) -> Result<()> {
    let status = Command::new("tar")
        .args(["-cJf", &output_path.to_string_lossy(), "-C", &dir.to_string_lossy(), "."])
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("Failed to create tar.xz archive with system tar"))
    }
}

/// Creates a 7Z archive from directory contents using system 7z.
pub fn create_7z_from_dir_contents(dir: &Path, output_path: &Path) -> Result<()> {
    let status = Command::new("7z")
        .args(["a", &output_path.to_string_lossy(), &format!("{}/*", dir.display())])
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("Failed to create 7z archive with system 7z"))
    }
}
