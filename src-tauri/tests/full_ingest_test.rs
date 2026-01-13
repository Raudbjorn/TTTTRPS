//! Full ingestion test - extract PDF and index to Meilisearch

use std::path::Path;

#[tokio::test(flavor = "multi_thread")]
#[ignore] // Run with: cargo test --test full_ingest_test -- --ignored --nocapture
async fn test_full_ingest() {
    let _ = env_logger::builder().is_test(true).try_init();

    let pdf_path = Path::new("/home/svnbjrn/Delta-Green-Agents-Handbook.pdf");
    if !pdf_path.exists() {
        println!("Test PDF not found");
        return;
    }

    println!("=== Full Ingestion Test ===\n");

    // 1. Extract (run in blocking task to avoid runtime conflicts)
    println!("[1/3] Extracting PDF (async/parallel page extraction)...");
    let path = pdf_path.to_path_buf();

    use ttrpg_assistant::ingestion::DocumentExtractor;
    let extractor = DocumentExtractor::with_ocr();
    let cb: Option<fn(f32, &str)> = None;
    let extracted = extractor.extract(&path, cb)
        .await
        .expect("Extraction failed");

    println!("Extracted {} chars from {} pages", extracted.char_count, extracted.page_count);

    // 2. Connect to Meilisearch
    println!("\n[2/3] Connecting to Meilisearch...");
    use ttrpg_assistant::core::search_client::SearchClient;

    let meili_key = std::fs::read_to_string("/etc/meilisearch.conf")
        .ok()
        .and_then(|content| {
            content.lines()
                .find(|l| l.starts_with("MEILI_MASTER_KEY="))
                .map(|l| l.trim_start_matches("MEILI_MASTER_KEY=").trim_matches('"').to_string())
        })
        .unwrap_or_else(|| "ttrpg-assistant-dev-key".to_string());

    let search_client = SearchClient::new("http://127.0.0.1:7700", Some(&meili_key));

    // 3. Ingest
    println!("\n[3/3] Ingesting to Meilisearch...");
    use ttrpg_assistant::core::meilisearch_pipeline::MeilisearchPipeline;

    let pipeline = MeilisearchPipeline::with_defaults();
    let result = pipeline.ingest_text(
        &search_client,
        &extracted.content,
        "Delta Green Agent's Handbook",
        "rulebook",
        None,
        None,
    ).await.expect("Ingestion failed");

    println!("Ingestion complete!");
    println!("  Source: {}", result.source);
    println!("  Total chunks: {}", result.total_chunks);
    println!("  Stored: {}", result.stored_chunks);
    println!("  Index: {}", result.index_used);

    // 4. Verify with search
    println!("\n[4/4] Verifying with search...");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await; // Let Meilisearch index

    let results = search_client.search("documents", "sanity points willpower", 5, None)
        .await
        .expect("Search failed");

    println!("Search returned {} results:", results.len());
    for (i, r) in results.iter().take(3).enumerate() {
        let preview: String = r.document.content.chars().take(150).collect();
        println!("  [{}] {}...", i + 1, preview);
    }

    println!("\n=== Test Complete ===");
}
