//! Meilisearch Integration Tests (Embedded Library)
//!
//! These tests use the embedded meilisearch-lib and do not require an external instance.
//! Run with: cargo test meilisearch_integration_tests

#[cfg(test)]
mod tests {
    use crate::core::search::EmbeddedSearch;
    use crate::core::meilisearch_pipeline::MeilisearchPipeline;
    use meilisearch_lib::SearchQuery;
    use tempfile::TempDir;

    /// Test helper to create a temporary embedded instance
    fn create_test_search() -> (TempDir, EmbeddedSearch) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("meilisearch");
        let search = EmbeddedSearch::new(db_path).expect("Failed to create embedded search");
        (temp_dir, search)
    }

    // ========================================================================
    // Health & Connection Tests
    // ========================================================================

    #[test]
    fn test_embedded_health_check() {
        let (_temp, search) = create_test_search();
        let health = search.inner().health();
        assert_eq!(health.status, "available", "Embedded Meilisearch should be available");
    }

    #[test]
    fn test_create_and_list_indexes() {
        let (_temp, search) = create_test_search();
        let meili = search.inner();

        // Create an index
        meili.create_index("test_index", Some("id"))
            .expect("Should create index");

        // Verify it exists
        assert!(meili.index_exists("test_index"), "Should find test_index");

        // Delete it
        meili.delete_index("test_index").expect("Should delete index");
        assert!(!meili.index_exists("test_index"), "test_index should be gone");
    }

    // ========================================================================
    // Document Indexing Tests
    // ========================================================================

    #[test]
    fn test_add_and_retrieve_document() {
        let (_temp, search) = create_test_search();
        let meili = search.inner();

        // Create index
        meili.create_index("test_docs", Some("id"))
            .expect("Should create index");

        // Add document
        let doc = serde_json::json!({
            "id": "doc1",
            "content": "The dragon breathes fire at the adventurers.",
            "source": "test_rules.pdf",
            "source_type": "rules"
        });
        let index = meili.get_index("test_docs").expect("Should get index");
        index.add_documents(vec![doc], Some("id")).expect("Should add document");

        // Search for it
        let query = SearchQuery::new("dragon fire").with_offset(0).with_limit(10);
        let results = index.search(&query).expect("Search should succeed");
        assert!(!results.hits.is_empty(), "Should find the dragon document");

        // Cleanup
        meili.delete_index("test_docs").expect("Should delete index");
    }

    // ========================================================================
    // Typo Tolerance Tests
    // ========================================================================

    #[test]
    fn test_typo_tolerance_single_typo() {
        let (_temp, search) = create_test_search();
        let meili = search.inner();

        // Create index
        meili.create_index("typo_test", Some("id"))
            .expect("Should create index");

        // Add document
        let doc = serde_json::json!({
            "id": "typo1",
            "content": "Fireball is a powerful evocation spell that deals fire damage."
        });
        let index = meili.get_index("typo_test").expect("Should get index");
        index.add_documents(vec![doc], Some("id")).expect("Should add document");

        // Search with typo: "firebll" instead of "fireball"
        let query = SearchQuery::new("firebll spell").with_offset(0).with_limit(10);
        let results = index.search(&query).expect("Search should succeed");
        assert!(!results.hits.is_empty(), "Should find 'fireball' even with typo 'firebll'");

        // Cleanup
        meili.delete_index("typo_test").expect("Should delete index");
    }

    // ========================================================================
    // Pipeline Tests
    // ========================================================================

    #[tokio::test]
    async fn test_pipeline_two_phase_ingestion() {
        let (_temp, search) = create_test_search();
        let meili = search.inner();

        let pipeline = MeilisearchPipeline::with_defaults();

        // Create a temporary test file with enough content to meet minimum chunk size (100 chars)
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_document.txt");
        let test_content = "This is a test document about dragons and treasure. Dragons love gold and gems. \
            The ancient wyrm guards a vast hoard of magical artifacts in its mountain lair. \
            Adventurers who seek this treasure must face many perils including traps and minions.";
        std::fs::write(&test_file, test_content).unwrap();

        // Use two-phase pipeline for ingestion
        let result = pipeline.ingest_two_phase(meili, &test_file, None).await;
        assert!(result.is_ok(), "Should process text file with two-phase pipeline: {:?}", result.err());

        let (extraction, chunking) = result.unwrap();
        assert!(extraction.page_count > 0, "Should extract at least one page");
        assert!(chunking.chunk_count > 0, "Should create at least one chunk");

        // Clean up test file
        std::fs::remove_file(&test_file).ok();

        // Verify it's searchable in the chunks index
        let chunks_index = meili.get_index(&chunking.chunks_index).expect("Should get chunks index");
        let query = SearchQuery::new("dragons treasure").with_offset(0).with_limit(10);
        let search_results = chunks_index.search(&query).expect("Search should succeed");
        assert!(!search_results.hits.is_empty(), "Should find ingested document in chunks index");

        // Clean up indexes
        let _ = meili.delete_index(&extraction.raw_index);
        let _ = meili.delete_index(&chunking.chunks_index);
    }

    // ========================================================================
    // Stats Tests
    // ========================================================================

    #[test]
    fn test_get_index_stats() {
        let (_temp, search) = create_test_search();
        let meili = search.inner();

        // Create index and add a document
        meili.create_index("stats_test", Some("id"))
            .expect("Should create index");

        let doc = serde_json::json!({"id": "s1", "content": "test"});
        let index = meili.get_index("stats_test").expect("Should get index");
        index.add_documents(vec![doc], Some("id")).expect("Should add document");

        // Get stats
        let stats = meili.index_stats("stats_test").expect("Should get stats");
        assert_eq!(stats.number_of_documents, 1, "Should have 1 document");

        // Cleanup
        meili.delete_index("stats_test").expect("Should delete index");
    }

    // ========================================================================
    // Clear Index Test
    // ========================================================================

    #[test]
    fn test_clear_index() {
        let (_temp, search) = create_test_search();
        let meili = search.inner();

        // Create index
        meili.create_index("clear_test", Some("id"))
            .expect("Should create index");

        // Add a document
        let doc = serde_json::json!({"id": "c1", "content": "This will be cleared"});
        let index = meili.get_index("clear_test").expect("Should get index");
        index.add_documents(vec![doc], Some("id")).expect("Should add document");

        // Clear the index by deleting and recreating
        meili.delete_index("clear_test").expect("Should delete index");
        meili.create_index("clear_test", Some("id")).expect("Should recreate index");

        // Verify it's empty
        let stats = meili.index_stats("clear_test").expect("Should get stats");
        assert_eq!(stats.number_of_documents, 0, "Index should be empty after clear");

        // Cleanup
        meili.delete_index("clear_test").expect("Should delete index");
    }
}
