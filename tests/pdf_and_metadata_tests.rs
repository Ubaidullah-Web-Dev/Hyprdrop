use std::fs;
use lopdf::{Document, Object, Stream, dictionary};
use hyprdrop::engine::metadata::{inspect_file_metadata, strip_file_metadata};
use hyprdrop::engine::pdf_ops::{convert_pdf_to, inspect_pdf_metadata, strip_pdf_metadata};
use tempfile::tempdir;

fn create_sample_pdf(path: &std::path::Path, title: &str, author: &str) {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();

    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });

    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! {
            "F1" => font_id,
        },
    });

    let content = "BT /F1 18 Tf 50 700 Td (Hello Hyprdrop PDF Engine) Tj ET";
    let content_id = doc.add_object(Stream::new(dictionary! {}, content.as_bytes().to_vec()));

    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => content_id,
    });

    let pages = dictionary! {
        "Type" => "Pages",
        "Kids" => vec![page_id.into()],
        "Count" => 1,
        "Resources" => resources_id,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
    };
    doc.objects.insert(pages_id, Object::Dictionary(pages));

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);

    let info_id = doc.add_object(dictionary! {
        "Title" => Object::string_literal(title),
        "Author" => Object::string_literal(author),
        "Producer" => Object::string_literal("Hyprdrop Test Suite"),
    });
    doc.trailer.set("Info", info_id);

    doc.save(path).expect("Failed to save sample PDF");
}

#[test]
fn test_pdf_metadata_inspection_and_stripping() {
    let dir = tempdir().expect("Failed to create tempdir");
    let pdf_path = dir.path().join("sample_doc.pdf");
    create_sample_pdf(&pdf_path, "Secret Document", "Arch Architect");

    // Inspect
    let meta = inspect_pdf_metadata(&pdf_path).expect("Failed to inspect metadata");
    assert_eq!(meta.get("Title").map(|s| s.as_str()), Some("Secret Document"));
    assert_eq!(meta.get("Author").map(|s| s.as_str()), Some("Arch Architect"));

    // Strip
    let stripped_path = dir.path().join("sample_doc_stripped.pdf");
    strip_pdf_metadata(&pdf_path, &stripped_path).expect("Failed to strip PDF metadata");

    // Test high-level facade
    let facade_meta = inspect_file_metadata(&pdf_path).expect("Failed facade inspect");
    assert!(facade_meta.fields.contains_key("Title"));

    let sanitized_path = strip_file_metadata(&pdf_path).expect("Failed facade strip");
    assert!(sanitized_path.exists());
    let sanitized_meta = inspect_file_metadata(&sanitized_path).expect("Failed facade re-inspect");
    assert!(!sanitized_meta.fields.contains_key("Title"));
}

#[test]
fn test_pdf_to_txt_and_html_conversion() {
    let dir = tempdir().expect("Failed to create tempdir");
    let pdf_path = dir.path().join("text_doc.pdf");
    create_sample_pdf(&pdf_path, "Text Report", "Tester");

    // Convert to TXT
    let txt_path = dir.path().join("text_doc.txt");
    convert_pdf_to(&pdf_path, &txt_path, "txt").expect("Failed to convert to TXT");
    assert!(txt_path.exists());

    // Convert to HTML
    let html_path = dir.path().join("text_doc.html");
    convert_pdf_to(&pdf_path, &html_path, "html").expect("Failed to convert to HTML");
    assert!(html_path.exists());
    let html_content = fs::read_to_string(&html_path).expect("Failed to read html");
    assert!(html_content.contains("<!DOCTYPE html>"));
}
