//! Integration test for PDF ingestion with kreuzberg + OCR fallback
//!
//! Run with: TEST_PDF_PATH=/path/to/test.pdf cargo test --test ingest_pdf_test -- --ignored

use std::path::Path;

#[tokio::test]
#[ignore = "Requires TEST_PDF_PATH environment variable pointing to a PDF file"]
async fn test_ingest_pdf() {
    // Initialize logger
    let _ = env_logger::builder().is_test(true).try_init();

    let pdf_path_str = match std::env::var("TEST_PDF_PATH") {
        Ok(path) => path,
        Err(_) => {
            println!("TEST_PDF_PATH not set, skipping test");
            println!("Run with: TEST_PDF_PATH=/path/to/test.pdf cargo test --test ingest_pdf_test -- --ignored");
            return;
        }
    };
    let pdf_path = Path::new(&pdf_path_str);
    if !pdf_path.exists() {
        println!("Test PDF not found at {:?}", pdf_path);
        return;
    }

    println!("=== Starting PDF ingestion test ===");
    println!("PDF: {:?}", pdf_path);

    // 1. Extract with kreuzberg (will fall back to OCR for scanned PDFs)
    use ttrpg_assistant::ingestion::DocumentExtractor;

    let extractor = DocumentExtractor::with_ocr();
    println!("\n[1/2] Extracting document...");

    let start = std::time::Instant::now();
    // Pass None explicitly for callback, specifying the type to help inference
    let cb: Option<fn(f32, &str)> = None;
    let extracted = extractor.extract(pdf_path, cb).await.expect("Extraction failed");
    let extract_time = start.elapsed();

    println!("Extraction complete in {:.1}s", extract_time.as_secs_f64());
    println!("  Pages: {}", extracted.page_count);
    println!("  Characters: {}", extracted.char_count);
    println!("  MIME type: {}", extracted.mime_type);

    // Show a sample of extracted text
    if extracted.char_count > 0 {
        let preview_len = extracted.content.len().min(2000);
        println!("\n=== Sample content (first {} chars) ===", preview_len);
        println!("{}", &extracted.content[..preview_len]);
    }

    assert!(extracted.char_count > 10000, "Expected substantial text content, got {}", extracted.char_count);
    assert!(extracted.page_count > 50, "Expected many pages, got {}", extracted.page_count);

    println!("\n=== Extraction test complete ===");
    println!("Total extraction time: {:.1}s", extract_time.as_secs_f64());
    println!("Ready for Meilisearch ingestion via app UI");
}
