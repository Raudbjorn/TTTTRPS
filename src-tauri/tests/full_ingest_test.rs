//! Full ingestion test - extract PDF and index to Meilisearch using two-phase pipeline
//!
//! Run with:
//!   TEST_PDF_PATH=/path/to/test.pdf MEILI_MASTER_KEY=your-key \
//!   cargo test --test full_ingest_test -- --ignored

use std::path::Path;

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires TEST_PDF_PATH env var and running Meilisearch instance"]
async fn test_full_ingest() {
    let _ = env_logger::builder().is_test(true).try_init();

    let pdf_path_str = match std::env::var("TEST_PDF_PATH") {
        Ok(path) => path,
        Err(_) => {
            println!("TEST_PDF_PATH not set, skipping test");
            return;
        }
    };
    let pdf_path = Path::new(&pdf_path_str);
    if !pdf_path.exists() {
        println!("Test PDF not found at {:?}", pdf_path);
        return;
    }

    println!("=== Full Ingestion Test (Two-Phase Pipeline) ===\n");

    // 1. Connect to Meilisearch
    println!("[1/3] Connecting to Meilisearch...");
    use ttrpg_assistant::core::search::SearchClient;

    let meili_key = std::env::var("MEILI_MASTER_KEY")
        .unwrap_or_else(|_| "ttrpg-assistant-dev-key".to_string());

    let search_client = SearchClient::new("http://127.0.0.1:7700", Some(&meili_key));

    // 2. Ingest using two-phase pipeline (extract → raw index → chunk index)
    println!("\n[2/3] Ingesting with two-phase pipeline...");
    use ttrpg_assistant::core::meilisearch_pipeline::MeilisearchPipeline;

    let pipeline = MeilisearchPipeline::with_defaults();
    let (extraction, chunking) = pipeline.ingest_two_phase(
        &search_client,
        pdf_path,
        Some("Delta Green Agent's Handbook"),
    ).await.expect("Two-phase ingestion failed");

    println!("Ingestion complete!");
    println!("  Slug: {}", extraction.slug);
    println!("  Source: {}", extraction.source_name);
    println!("  Pages: {}", extraction.page_count);
    println!("  Characters: {}", extraction.total_chars);
    println!("  Chunks: {}", chunking.chunk_count);
    println!("  Raw index: {}", extraction.raw_index);
    println!("  Chunks index: {}", chunking.chunks_index);
    if let Some(system) = &extraction.ttrpg_metadata.game_system {
        println!("  Game system: {}", system);
    }

    // 3. Verify with search
    println!("\n[3/3] Verifying with search...");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await; // Let Meilisearch index

    // Search the chunks index
    let results = search_client.search(&chunking.chunks_index, "sanity points willpower", 5, None)
        .await
        .expect("Search failed");

    println!("Search returned {} results:", results.len());
    for (i, r) in results.iter().take(3).enumerate() {
        let preview: String = r.document.content.chars().take(150).collect();
        println!("  [{}] {}...", i + 1, preview);
    }

    println!("\n=== Test Complete ===");
}
