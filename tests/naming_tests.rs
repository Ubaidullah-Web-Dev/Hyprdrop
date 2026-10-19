use std::fs::File;
use hyprdrop::engine::naming::{
    get_collision_safe_path, get_converted_output_path, get_suffixed_output_path,
};
use tempfile::tempdir;

#[test]
fn test_collision_safe_non_existing() {
    let dir = tempdir().expect("Failed to create tempdir");
    let target = dir.path().join("unique_file.png");

    let safe = get_collision_safe_path(&target);
    assert_eq!(safe, target);
}

#[test]
fn test_collision_safe_existing_files() {
    let dir = tempdir().expect("Failed to create tempdir");
    let target = dir.path().join("document.pdf");
    File::create(&target).expect("Failed to create test file");

    let safe1 = get_collision_safe_path(&target);
    assert_eq!(safe1, dir.path().join("document (2).pdf"));

    // Create document (2).pdf and ensure next is (3)
    File::create(&safe1).expect("Failed to create safe1");
    let safe2 = get_collision_safe_path(&target);
    assert_eq!(safe2, dir.path().join("document (3).pdf"));
}

#[test]
fn test_collision_safe_existing_counter_in_name() {
    let dir = tempdir().expect("Failed to create tempdir");
    let target = dir.path().join("image (1).png");
    File::create(&target).expect("Failed to create test file");

    let safe = get_collision_safe_path(&target);
    // Should become image (2).png rather than image (1) (2).png
    assert_eq!(safe, dir.path().join("image (2).png"));
}

#[test]
fn test_unicode_and_spaces() {
    let dir = tempdir().expect("Failed to create tempdir");
    let target = dir.path().join("日本語 ファイル 🚀.png");
    File::create(&target).expect("Failed to create test file");

    let safe = get_collision_safe_path(&target);
    assert_eq!(safe, dir.path().join("日本語 ファイル 🚀 (2).png"));
}

#[test]
fn test_no_extension() {
    let dir = tempdir().expect("Failed to create tempdir");
    let target = dir.path().join("Makefile");
    File::create(&target).expect("Failed to create test file");

    let safe = get_collision_safe_path(&target);
    assert_eq!(safe, dir.path().join("Makefile (2)"));
}

#[test]
fn test_converted_output_path() {
    let dir = tempdir().expect("Failed to create tempdir");
    let src = dir.path().join("presentation.pdf");
    File::create(&src).expect("Failed to create file");

    let out = get_converted_output_path(&src, "png");
    assert_eq!(out, dir.path().join("presentation.png"));

    // If destination already exists
    File::create(&out).expect("Failed to create out");
    let out2 = get_converted_output_path(&src, "png");
    assert_eq!(out2, dir.path().join("presentation (2).png"));
}

#[test]
fn test_suffixed_output_path() {
    let dir = tempdir().expect("Failed to create tempdir");
    let src = dir.path().join("photo.jpg");
    File::create(&src).expect("Failed to create file");

    let out = get_suffixed_output_path(&src, "Cropped", None);
    assert_eq!(out, dir.path().join("photo Cropped.jpg"));

    let out_bg = get_suffixed_output_path(&src, "With Background", Some("png"));
    assert_eq!(out_bg, dir.path().join("photo With Background.png"));
}
