//! Meilisearch Integration Tests
//!
//! These tests require a running Meilisearch instance.
//! Run with: cargo test -- --ignored

#[cfg(test)]
mod tests {
    use crate::core::search_client::{SearchClient, SearchDocument, INDEX_RULES, INDEX_DOCUMENTS};
    use crate::core::meilisearch_pipeline::MeilisearchPipeline;
    use std::collections::HashMap;

    const TEST_HOST: &str = "http://127.0.0.1:7700";
    const TEST_KEY: &str = "ttrpg-assistant-dev-key";

    /// Test helper to create a test client
    fn test_client() -> SearchClient {
        SearchClient::new(TEST_HOST, Some(TEST_KEY))
    }

    /// Helper to create a test document
    fn test_doc(id: &str, content: &str, source: &str, source_type: &str) -> SearchDocument {
        SearchDocument {
            id: id.to_string(),
            content: content.to_string(),
            source: source.to_string(),
            source_type: source_type.to_string(),
            page_number: None,
            chunk_index: Some(0),
            campaign_id: None,
            session_id: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            metadata: HashMap::new(),
            ..Default::default()
        }
    }

    // ========================================================================
    // Health & Connection Tests
    // ========================================================================

    #[tokio::test]
    #[ignore = "Requires running Meilisearch instance"]
    async fn test_meilisearch_health_check() {
        let client = test_client();
        let healthy = client.health_check().await;
        assert!(healthy, "Meilisearch should be healthy and responding");
    }

    #[tokio::test]
    #[ignore = "Requires running Meilisearch instance"]
    async fn test_initialize_indexes() {
        let client = test_client();
        let result = client.initialize_indexes().await;
        assert!(result.is_ok(), "Should initialize all indexes: {:?}", result.err());
    }

    // ========================================================================
    // Document Indexing Tests
    // ========================================================================

    #[tokio::test]
    #[ignore = "Requires running Meilisearch instance"]
    async fn test_add_and_retrieve_document() {
        let client = test_client();
        client.initialize_indexes().await.unwrap();

        let doc = test_doc(
            "test-doc-1",
            "The dragon breathes fire at the adventurers.",
            "test_rules.pdf",
            "rules"
        );

        // Add document
        let result = client.add_documents(INDEX_RULES, vec![doc]).await;
        assert!(result.is_ok(), "Should add document: {:?}", result.err());

        // Wait for indexing
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // Search for it
        let results = client.search(INDEX_RULES, "dragon fire", 10, None).await;
        assert!(results.is_ok(), "Search should succeed");
        let results = results.unwrap();
        assert!(!results.is_empty(), "Should find the dragon document");
        assert!(results[0].document.content.contains("dragon"));
    }

    // ========================================================================
    // Typo Tolerance Tests
    // ========================================================================

    #[tokio::test]
    #[ignore = "Requires running Meilisearch instance"]
    async fn test_typo_tolerance_single_typo() {
        let client = test_client();
        client.initialize_indexes().await.unwrap();

        let doc = test_doc(
            "typo-test-1",
            "Fireball is a powerful evocation spell that deals fire damage.",
            "spells.pdf",
            "rules"
        );
        client.add_documents(INDEX_RULES, vec![doc]).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // Search with typo: "firebll" instead of "fireball"
        let results = client.search(INDEX_RULES, "firebll spell", 10, None).await.unwrap();
        assert!(!results.is_empty(), "Should find 'fireball' even with typo 'firebll'");
    }

    #[tokio::test]
    #[ignore = "Requires running Meilisearch instance"]
    async fn test_typo_tolerance_two_typos() {
        let client = test_client();
        client.initialize_indexes().await.unwrap();

        let doc = test_doc(
            "typo-test-2",
            "Constitution saving throw against poison damage.",
            "rules.pdf",
            "rules"
        );
        client.add_documents(INDEX_RULES, vec![doc]).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // Search with typos: "consitution" (missing 't') and "poisen" (wrong vowel)
        let results = client.search(INDEX_RULES, "consitution poisen", 10, None).await.unwrap();
        assert!(!results.is_empty(), "Should find document with multiple typos");
    }

    // ========================================================================
    // Federated Search Tests
    // ========================================================================

    #[tokio::test]
    #[ignore = "Requires running Meilisearch instance"]
    async fn test_federated_search_across_indexes() {
        let client = test_client();
        client.initialize_indexes().await.unwrap();

        // Add to rules index
        let rules_doc = test_doc(
            "fed-rules-1",
            "Elves have darkvision up to 60 feet.",
            "phb.pdf",
            "rules"
        );
        client.add_documents(INDEX_RULES, vec![rules_doc]).await.unwrap();

        // Add to documents index
        let doc = test_doc(
            "fed-doc-1",
            "The elven city was hidden in the forest.",
            "notes.md",
            "document"
        );
        client.add_documents(INDEX_DOCUMENTS, vec![doc]).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // Federated search should find both
        let results = client.search_all("elves", 10).await.unwrap();
        assert!(results.results.len() >= 2, "Should find documents from multiple indexes");

        // Check we got results from different indexes
        let indexes: std::collections::HashSet<_> = results.results.iter().map(|r| r.index.as_str()).collect();
        assert!(indexes.len() >= 2, "Results should come from multiple indexes");
    }

    // ========================================================================
    // Pipeline Tests
    // ========================================================================

    #[tokio::test]
    #[ignore = "Requires running Meilisearch instance"]
    async fn test_pipeline_text_ingestion() {
        let client = test_client();
        client.initialize_indexes().await.unwrap();

        let pipeline = MeilisearchPipeline::with_defaults();

        // Create a temporary test file
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_document.txt");
        std::fs::write(&test_file, "This is a test document about dragons and treasure.").unwrap();

        // Process the file
        let result = pipeline.process_file(&client, &test_file, "document", None).await;
        assert!(result.is_ok(), "Should process text file: {:?}", result.err());

        let result = result.unwrap();
        assert!(result.stored_chunks > 0, "Should store at least one chunk");

        // Clean up
        std::fs::remove_file(test_file).ok();

        // Verify it's searchable
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let search_results = client.search(INDEX_DOCUMENTS, "dragons treasure", 10, None).await.unwrap();
        assert!(!search_results.is_empty(), "Should find ingested document");
    }

    // ========================================================================
    // Stats Tests
    // ========================================================================

    #[tokio::test]
    #[ignore = "Requires running Meilisearch instance"]
    async fn test_get_all_stats() {
        let client = test_client();
        client.initialize_indexes().await.unwrap();

        let stats = client.get_all_stats().await;
        assert!(stats.is_ok(), "Should get stats: {:?}", stats.err());

        let stats = stats.unwrap();
        // Should have entries for our indexes
        assert!(stats.contains_key(INDEX_RULES), "Should have rules index stats");
        assert!(stats.contains_key(INDEX_DOCUMENTS), "Should have documents index stats");
    }

    // ========================================================================
    // Cleanup Helper
    // ========================================================================

    #[tokio::test]
    #[ignore = "Requires running Meilisearch instance"]
    async fn test_clear_index() {
        let client = test_client();
        client.initialize_indexes().await.unwrap();

        // Add a document
        let doc = test_doc(
            "clear-test-1",
            "This document will be cleared.",
            "temp.txt",
            "document"
        );
        client.add_documents(INDEX_DOCUMENTS, vec![doc]).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // Clear the index
        let result = client.clear_index(INDEX_DOCUMENTS).await;
        assert!(result.is_ok(), "Should clear index");

        // Verify it's empty
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let stats = client.get_all_stats().await.unwrap();
        assert_eq!(stats.get(INDEX_DOCUMENTS).copied().unwrap_or(0), 0, "Index should be empty after clear");
    }
}
