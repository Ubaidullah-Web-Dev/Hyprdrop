use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use anyhow::{Result, anyhow};

/// Extracts archive into its parent directory or a dedicated folder.
pub fn extract_archive(archive_path: &Path, destination_dir: &Path) -> Result<PathBuf> {
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
    } else {
        Err(anyhow!("Unsupported archive format: {}", file_name))
    }
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
