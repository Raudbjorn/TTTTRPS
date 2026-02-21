//! Full ingestion test - extract PDF and index to embedded Meilisearch using two-phase pipeline
//!
//! Run with:
//!   TEST_PDF_PATH=/path/to/test.pdf cargo test --test full_ingest_test -- --ignored

use std::path::Path;
use tempfile::TempDir;

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires TEST_PDF_PATH env var"]
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

    // 1. Create temporary embedded Meilisearch instance
    println!("[1/3] Creating embedded Meilisearch instance...");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("meilisearch");

    use ttrpg_assistant::core::search::EmbeddedSearch;
    let search = EmbeddedSearch::new(db_path).expect("Failed to create embedded search");
    let meili = search.inner();

    // 2. Ingest using two-phase pipeline (extract → raw index → chunk index)
    println!("\n[2/3] Ingesting with two-phase pipeline...");
    use ttrpg_assistant::core::meilisearch_pipeline::MeilisearchPipeline;

    let pipeline = MeilisearchPipeline::with_defaults();
    let (extraction, chunking) = pipeline.ingest_two_phase(
        meili,
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

    // Give embedded Meilisearch time to process
    std::thread::sleep(std::time::Duration::from_secs(1));

    // Search the chunks index
    use meilisearch_lib::SearchQuery;
    let query = SearchQuery::new("sanity points willpower")
        .with_pagination(0, 5);

    let results = meili.search(&chunking.chunks_index, query)
        .expect("Search failed");

    println!("Search returned {} results:", results.hits.len());
    for (i, hit) in results.hits.iter().take(3).enumerate() {
        if let Some(content) = hit.document.get("content").and_then(|v| v.as_str()) {
            let preview: String = content.chars().take(150).collect();
            println!("  [{}] {}...", i + 1, preview);
        }
    }

    // Cleanup
    search.shutdown().expect("Shutdown should succeed");

    println!("\n=== Test Complete ===");
}
