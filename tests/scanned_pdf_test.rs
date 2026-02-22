//! Integration test for scanned/image-only PDF ingestion with OCR
//!
//! These tests use fixture PDFs that contain only images (no embedded text),
//! requiring OCR extraction to retrieve content.
//!
//! Run with: cargo test --test scanned_pdf_test -- --ignored

use std::path::Path;

/// Test OCR extraction on the generated TTRPG-themed scanned document
#[tokio::test]
#[ignore = "Requires tesseract OCR installed"]
async fn test_scanned_ttrpg_document() {
    let _ = env_logger::builder().is_test(true).try_init();

    let pdf_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/scanned_document_test.pdf");

    if !pdf_path.exists() {
        println!("Fixture not found at {:?}, skipping", pdf_path);
        return;
    }

    println!("=== Scanned TTRPG Document OCR Test ===");
    println!("PDF: {:?}", pdf_path);

    use ttttrps::ingestion::DocumentExtractor;

    let extractor = DocumentExtractor::with_ocr();
    let cb: Option<fn(f32, &str)> = None;
    let extracted = extractor.extract(&pdf_path, cb).await.expect("Extraction failed");

    println!("Extraction complete:");
    println!("  Pages: {}", extracted.page_count);
    println!("  Characters: {}", extracted.char_count);

    // Verify basic extraction worked
    assert_eq!(extracted.page_count, 1, "Expected 1 page");
    assert!(extracted.char_count > 500, "Expected at least 500 characters, got {}", extracted.char_count);

    // Verify key content was extracted
    let content_lower = extracted.content.to_lowercase();
    assert!(content_lower.contains("ancient tome"), "Missing 'ancient tome' from paragraph 1");
    assert!(content_lower.contains("shadow drake"), "Missing 'Shadow Drake' from table");
    assert!(content_lower.contains("frost elemental"), "Missing 'Frost Elemental' from table");
    assert!(content_lower.contains("adventurers"), "Missing 'adventurers' from paragraph 2");
    assert!(content_lower.contains("initiative"), "Missing 'initiative' from paragraph 3");

    println!("\n=== Content sample ===");
    println!("{}", &extracted.content[..extracted.content.len().min(500)]);
    println!("\n=== Test passed ===");
}

/// Test OCR extraction on the lighthouse keeper document
#[tokio::test]
#[ignore = "Requires tesseract OCR installed"]
async fn test_scanned_lighthouse_document() {
    let _ = env_logger::builder().is_test(true).try_init();

    let pdf_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/image_only_document.pdf");

    if !pdf_path.exists() {
        println!("Fixture not found at {:?}, skipping", pdf_path);
        return;
    }

    println!("=== Lighthouse Document OCR Test ===");
    println!("PDF: {:?}", pdf_path);

    use ttttrps::ingestion::DocumentExtractor;

    let extractor = DocumentExtractor::with_ocr();
    let cb: Option<fn(f32, &str)> = None;
    let extracted = extractor.extract(&pdf_path, cb).await.expect("Extraction failed");

    println!("Extraction complete:");
    println!("  Pages: {}", extracted.page_count);
    println!("  Characters: {}", extracted.char_count);

    // Verify basic extraction worked
    assert_eq!(extracted.page_count, 1, "Expected 1 page");
    assert!(extracted.char_count > 1000, "Expected at least 1000 characters, got {}", extracted.char_count);

    // Verify key content was extracted
    let content_lower = extracted.content.to_lowercase();
    assert!(content_lower.contains("lighthouse"), "Missing 'lighthouse' from content");
    assert!(content_lower.contains("margaret"), "Missing 'Margaret' from content");
    assert!(content_lower.contains("herring"), "Missing 'herring' from content");
    assert!(content_lower.contains("reykjavik") || content_lower.contains("vestmannaeyjar"),
            "Missing Icelandic location names from table");

    println!("\n=== Content sample ===");
    println!("{}", &extracted.content[..extracted.content.len().min(500)]);
    println!("\n=== Test passed ===");
}

/// Test that both scanned documents can be processed sequentially
#[tokio::test]
#[ignore = "Requires tesseract OCR installed"]
async fn test_multiple_scanned_documents() {
    let _ = env_logger::builder().is_test(true).try_init();

    use ttttrps::ingestion::DocumentExtractor;

    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let extractor = DocumentExtractor::with_ocr();

    let pdfs = [
        "scanned_document_test.pdf",
        "image_only_document.pdf",
    ];

    println!("=== Multiple Scanned Documents Test ===\n");

    for pdf_name in pdfs {
        let pdf_path = fixtures_dir.join(pdf_name);
        if !pdf_path.exists() {
            println!("Skipping {}: not found", pdf_name);
            continue;
        }

        println!("Processing: {}", pdf_name);
        let cb: Option<fn(f32, &str)> = None;
        let extracted = extractor.extract(&pdf_path, cb).await
            .expect(&format!("Failed to extract {}", pdf_name));

        println!("  Pages: {}, Characters: {}", extracted.page_count, extracted.char_count);
        assert!(extracted.char_count > 100, "Extraction returned too little content for {}", pdf_name);
    }

    println!("\n=== All documents processed successfully ===");
}
