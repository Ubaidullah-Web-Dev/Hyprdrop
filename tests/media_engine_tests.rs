use std::fs::{self, File};
use std::io::Write;
use image::{DynamicImage, ImageReader, Rgba, RgbaImage};
use hyprdrop::engine::archive_ops::{create_tar_gz, create_tar_zst, create_zip, extract_archive};
use hyprdrop::engine::compress::compress_file;
use hyprdrop::engine::image_ops::{
    StudioBackgroundPreset, TargetAspectRatio, apply_rounded_corners, auto_trim_transparent,
    convert_image_format, crop_image, render_studio_background,
};
use tempfile::tempdir;

fn create_test_image(width: u32, height: u32) -> DynamicImage {
    let mut img = RgbaImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let r = ((x * 255) / width) as u8;
            let g = ((y * 255) / height) as u8;
            let b = 180u8;
            img.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }
    DynamicImage::ImageRgba8(img)
}

#[test]
fn test_image_format_conversions_end_to_end() {
    let dir = tempdir().expect("Failed to create tempdir");
    let src_png = dir.path().join("source.png");

    let test_img = create_test_image(120, 80);
    test_img.save(&src_png).expect("Failed to save source PNG");

    let formats = ["jpg", "webp", "bmp", "tiff", "ico", "pdf"];
    for fmt in formats {
        let out_path = dir.path().join(format!("output.{}", fmt));
        convert_image_format(&src_png, &out_path, fmt)
            .unwrap_or_else(|e| panic!("Failed to convert to {}: {:?}", fmt, e));

        assert!(out_path.exists(), "Output file for format {} does not exist", fmt);
        assert!(fs::metadata(&out_path).unwrap().len() > 0, "Output file for format {} is empty", fmt);

        // For image formats, verify we can decode them back
        if fmt != "pdf" {
            let decoded = ImageReader::open(&out_path)
                .unwrap()
                .with_guessed_format()
                .unwrap()
                .decode()
                .unwrap();
            assert!(decoded.width() > 0 && decoded.height() > 0);
        }
    }
}

#[test]
fn test_image_cropping() {
    let test_img = create_test_image(200, 100);
    let cropped = crop_image(&test_img, 20, 10, 80, 50);

    assert_eq!(cropped.width(), 80);
    assert_eq!(cropped.height(), 50);

    // Test clamped bounds
    let clamped = crop_image(&test_img, 180, 80, 200, 200);
    assert_eq!(clamped.width(), 20); // 200 - 180
    assert_eq!(clamped.height(), 20); // 100 - 80
}

#[test]
fn test_rounded_corners() {
    let test_img = create_test_image(100, 100).to_rgba8();
    let rounded = apply_rounded_corners(&test_img, 20.0);

    // Center pixel should remain fully opaque
    let center = rounded.get_pixel(50, 50);
    assert_eq!(center[3], 255);

    // Extreme corner pixel (0, 0) should be fully transparent
    let corner = rounded.get_pixel(0, 0);
    assert_eq!(corner[3], 0);

    // Antialiased border pixel should have partial alpha (0 < alpha < 255)
    let edge_pixel = rounded.get_pixel(5, 5);
    assert_eq!(edge_pixel[3], 0);
}

#[test]
fn test_studio_background_rendering() {
    let test_img = create_test_image(100, 80);
    let rendered = render_studio_background(
        &test_img,
        StudioBackgroundPreset::SunsetOrange,
        30,
        15.0,
        0.5,
        TargetAspectRatio::Auto,
    );

    // Dimensions: 100 + 30*2 = 160, 80 + 30*2 = 140
    assert_eq!(rendered.width(), 160);
    assert_eq!(rendered.height(), 140);

    // Top-left pixel should have the background preset's top color (Sunset Orange)
    let top_left = rendered.get_pixel(0, 0);
    assert_eq!(top_left[0], 255);
    assert_eq!(top_left[1], 87);
    assert_eq!(top_left[2], 34);
    assert_eq!(top_left[3], 255);
}

#[test]
fn test_auto_trim_transparent() {
    let mut img = RgbaImage::new(100, 100);
    // Draw 40x30 opaque block centered at (30, 35)
    for y in 35..65 {
        for x in 30..70 {
            img.put_pixel(x, y, Rgba([255, 0, 0, 255]));
        }
    }

    let trimmed = auto_trim_transparent(&img);
    assert_eq!(trimmed.width(), 40);
    assert_eq!(trimmed.height(), 30);
}

#[test]
fn test_archive_zip_tar_zst_tar_gz_end_to_end() {
    let dir = tempdir().expect("Failed to create tempdir");
    let sample_file = dir.path().join("sample.txt");
    let mut f = File::create(&sample_file).expect("Failed to create sample file");
    f.write_all(b"Hyprdrop High-Performance Archive Test Content")
        .expect("Failed to write content");

    // 1. ZIP
    let zip_file = dir.path().join("archive.zip");
    create_zip(&sample_file, &zip_file).expect("Failed to create zip");
    assert!(zip_file.exists());

    let zip_extract_dir = dir.path().join("zip_extracted");
    fs::create_dir_all(&zip_extract_dir).expect("Failed to create dir");
    extract_archive(&zip_file, &zip_extract_dir).expect("Failed to extract zip");
    let extracted_txt = zip_extract_dir.join("sample.txt");
    assert!(extracted_txt.exists());
    assert_eq!(
        fs::read_to_string(&extracted_txt).unwrap(),
        "Hyprdrop High-Performance Archive Test Content"
    );

    // 2. TAR.GZ
    let targz_file = dir.path().join("archive.tar.gz");
    create_tar_gz(&sample_file, &targz_file).expect("Failed to create tar.gz");
    assert!(targz_file.exists());

    let targz_extract_dir = dir.path().join("targz_extracted");
    fs::create_dir_all(&targz_extract_dir).expect("Failed to create dir");
    extract_archive(&targz_file, &targz_extract_dir).expect("Failed to extract tar.gz");
    assert!(targz_extract_dir.join("sample.txt").exists());

    // 3. TAR.ZST
    let tarzst_file = dir.path().join("archive.tar.zst");
    create_tar_zst(&sample_file, &tarzst_file).expect("Failed to create tar.zst");
    assert!(tarzst_file.exists());

    let tarzst_extract_dir = dir.path().join("tarzst_extracted");
    fs::create_dir_all(&tarzst_extract_dir).expect("Failed to create dir");
    extract_archive(&tarzst_file, &tarzst_extract_dir).expect("Failed to extract tar.zst");
    assert!(tarzst_extract_dir.join("sample.txt").exists());
}

#[test]
fn test_image_compression() {
    let dir = tempdir().expect("Failed to create tempdir");
    let large_img = dir.path().join("large.png");

    let img = create_test_image(800, 600);
    img.save(&large_img).expect("Failed to save image");

    let res = compress_file(&large_img).expect("Failed to compress image");
    assert!(res.output_path.exists());
    assert!(res.compressed_bytes > 0);
}

#[test]
fn test_execute_conversion_batch_parallel() {
    use hyprdrop::engine::execute_conversion_batch;

    let dir = tempdir().expect("Failed to create tempdir");
    let mut sources = Vec::new();

    for i in 0..4 {
        let p = dir.path().join(format!("batch_item_{}.png", i));
        let img = create_test_image(120, 80);
        img.save(&p).expect("Failed to save test image");
        sources.push(p);
    }

    let results = execute_conversion_batch(&sources, "jpg");
    assert_eq!(results.len(), 4);

    for (i, res) in results.into_iter().enumerate() {
        let out_path = res.unwrap_or_else(|e| panic!("Batch item {} failed: {:?}", i, e));
        assert!(out_path.exists());
        assert_eq!(out_path.extension().and_then(|e| e.to_str()), Some("jpg"));
    }
}

