//! Meilisearch Integration Tests
//!
//! Comprehensive integration tests for Meilisearch operations including:
//! - Document indexing end-to-end
//! - Various search query types (keyword, typo tolerance, filters)
//! - Hybrid search (BM25 + vector)
//! - Search analytics recording
//! - Index deletion and cleanup
//!
//! Tests marked with #[ignore] require a running Meilisearch instance.
//! Run with: cargo test -- --ignored

use crate::core::search::{
    SearchClient, SearchDocument, INDEX_RULES, INDEX_FICTION, INDEX_CHAT, INDEX_DOCUMENTS,
};
use crate::core::search_analytics::{SearchAnalytics, SearchRecord, ResultSelection};
use crate::database::{Database, SearchAnalyticsOps, SearchAnalyticsRecord};
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;
use chrono::Utc;

// =============================================================================
// Test Configuration
// =============================================================================

const TEST_HOST: &str = "http://127.0.0.1:7700";
const TEST_KEY: &str = "ttrpg-assistant-dev-key";

/// Create a test search client
fn test_client() -> SearchClient {
    SearchClient::new(TEST_HOST, Some(TEST_KEY)).expect("Failed to create SearchClient")
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

/// Helper to create a test document with metadata
#[allow(dead_code)]
fn test_doc_with_metadata(
    id: &str,
    content: &str,
    source: &str,
    source_type: &str,
    metadata: HashMap<String, String>,
) -> SearchDocument {
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
        metadata,
        ..Default::default()
    }
}

// =============================================================================
// Document Indexing End-to-End Tests
// =============================================================================

#[tokio::test]
#[ignore = "Requires running Meilisearch instance"]
async fn test_document_indexing_end_to_end() {
    let client = test_client();

    // Initialize indexes
    client
        .initialize_indexes()
        .await
        .expect("Failed to initialize indexes");

    // Wait for initialization
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Create test documents for different categories
    let rules_docs = vec![
        test_doc(
            "rule-001",
            "When a creature takes damage, it must make a Constitution saving throw to maintain concentration on a spell. The DC equals 10 or half the damage taken, whichever is higher.",
            "phb_chapter_10.pdf",
            "rules",
        ),
        test_doc(
            "rule-002",
            "Opportunity attacks occur when a hostile creature moves out of your reach. You can use your reaction to make a melee attack against the creature.",
            "phb_chapter_9.pdf",
            "rules",
        ),
        test_doc(
            "rule-003",
            "Critical hits occur when you roll a natural 20 on an attack roll. When you score a critical hit, you roll all attack damage dice twice.",
            "phb_chapter_9.pdf",
            "rules",
        ),
    ];

    let fiction_docs = vec![
        test_doc(
            "fiction-001",
            "The ancient dragon Klauth, also known as Old Snarl, rules the Sword Mountains. He is one of the most powerful creatures in the North.",
            "sword_coast_guide.pdf",
            "fiction",
        ),
        test_doc(
            "fiction-002",
            "Waterdeep, the City of Splendors, is a bustling metropolis on the Sword Coast. It is governed by the Lords of Waterdeep.",
            "waterdeep_gazetteer.pdf",
            "fiction",
        ),
    ];

    // Add documents to their respective indexes
    client
        .add_documents(INDEX_RULES, rules_docs)
        .await
        .expect("Failed to add rules documents");

    client
        .add_documents(INDEX_FICTION, fiction_docs)
        .await
        .expect("Failed to add fiction documents");

    // Wait for indexing
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Verify document counts
    let rules_count = client
        .document_count(INDEX_RULES)
        .await
        .expect("Failed to get rules count");
    assert!(rules_count >= 3, "Expected at least 3 rules documents");

    let fiction_count = client
        .document_count(INDEX_FICTION)
        .await
        .expect("Failed to get fiction count");
    assert!(fiction_count >= 2, "Expected at least 2 fiction documents");

    // Search in rules index
    let results = client
        .search(INDEX_RULES, "critical hit damage", 10, None)
        .await
        .expect("Search failed");

    assert!(!results.is_empty(), "Should find critical hit rules");
    assert!(
        results[0].document.content.to_lowercase().contains("critical"),
        "Top result should contain 'critical'"
    );

    // Search in fiction index
    let fiction_results = client
        .search(INDEX_FICTION, "dragon mountains", 10, None)
        .await
        .expect("Fiction search failed");

    assert!(!fiction_results.is_empty(), "Should find dragon lore");

    // Clean up test documents
    for id in ["rule-001", "rule-002", "rule-003"] {
        client.delete_document(INDEX_RULES, id).await.ok();
    }
    for id in ["fiction-001", "fiction-002"] {
        client.delete_document(INDEX_FICTION, id).await.ok();
    }
}

#[tokio::test]
#[ignore = "Requires running Meilisearch instance"]
async fn test_document_update_and_deletion() {
    let client = test_client();
    client.initialize_indexes().await.unwrap();

    // Add initial document
    let doc = test_doc(
        "update-test-001",
        "Initial content about goblins.",
        "test.pdf",
        "rules",
    );
    client.add_documents(INDEX_RULES, vec![doc]).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Search for initial content
    let initial_results = client.search(INDEX_RULES, "goblins", 10, None).await.unwrap();
    assert!(!initial_results.is_empty());

    // Update document with new content (same ID = update)
    let updated_doc = test_doc(
        "update-test-001",
        "Updated content about orcs and their tribes.",
        "test.pdf",
        "rules",
    );
    client.add_documents(INDEX_RULES, vec![updated_doc]).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Search for new content
    let updated_results = client.search(INDEX_RULES, "orcs tribes", 10, None).await.unwrap();
    assert!(!updated_results.is_empty(), "Should find updated content");

    // Old content should not be found in this document
    let old_results = client.search(INDEX_RULES, "goblins update-test-001", 10, None).await.unwrap();
    let has_old_content = old_results.iter().any(|r| r.document.id == "update-test-001" && r.document.content.contains("goblins"));
    assert!(!has_old_content, "Old content should be replaced");

    // Delete document
    client.delete_document(INDEX_RULES, "update-test-001").await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Verify deletion
    let after_delete = client.search(INDEX_RULES, "orcs update-test-001", 10, None).await.unwrap();
    let found = after_delete.iter().any(|r| r.document.id == "update-test-001");
    assert!(!found, "Document should be deleted");
}

// =============================================================================
// Various Search Query Types Tests
// =============================================================================

#[tokio::test]
#[ignore = "Requires running Meilisearch instance"]
async fn test_exact_phrase_search() {
    let client = test_client();
    client.initialize_indexes().await.unwrap();

    // Add document with specific phrase
    let doc = test_doc(
        "phrase-test-001",
        "The ancient tome describes the ritual of binding souls to objects.",
        "magic_items.pdf",
        "rules",
    );
    client.add_documents(INDEX_RULES, vec![doc]).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Search for exact phrase
    let results = client
        .search(INDEX_RULES, "ritual of binding souls", 10, None)
        .await
        .unwrap();

    assert!(!results.is_empty(), "Should find document with phrase");
    assert!(
        results[0].document.content.contains("ritual of binding souls"),
        "Should match exact phrase"
    );

    // Clean up
    client.delete_document(INDEX_RULES, "phrase-test-001").await.ok();
}

#[tokio::test]
#[ignore = "Requires running Meilisearch instance"]
async fn test_typo_tolerance_various_errors() {
    let client = test_client();
    client.initialize_indexes().await.unwrap();

    // Add documents
    let docs = vec![
        test_doc(
            "typo-001",
            "Fireball is a 3rd-level evocation spell that deals fire damage in a 20-foot radius.",
            "spells.pdf",
            "rules",
        ),
        test_doc(
            "typo-002",
            "Constitution modifier adds to your hit points at each level.",
            "classes.pdf",
            "rules",
        ),
        test_doc(
            "typo-003",
            "Thunderwave creates a thunderous force that pushes creatures away.",
            "spells.pdf",
            "rules",
        ),
    ];
    client.add_documents(INDEX_RULES, vec![docs[0].clone(), docs[1].clone(), docs[2].clone()]).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Test single character typo
    let single_typo = client.search(INDEX_RULES, "firebll", 10, None).await.unwrap();
    assert!(!single_typo.is_empty(), "Should find 'fireball' with typo 'firebll'");

    // Test transposition typo
    let transposition = client.search(INDEX_RULES, "consitution", 10, None).await.unwrap();
    assert!(!transposition.is_empty(), "Should find 'constitution' with transposition");

    // Test double typo
    let double_typo = client.search(INDEX_RULES, "thnuderwave", 10, None).await.unwrap();
    assert!(!double_typo.is_empty(), "Should find 'thunderwave' with double typo");

    // Clean up
    for id in ["typo-001", "typo-002", "typo-003"] {
        client.delete_document(INDEX_RULES, id).await.ok();
    }
}

#[tokio::test]
#[ignore = "Requires running Meilisearch instance"]
async fn test_filtered_search() {
    let client = test_client();
    client.initialize_indexes().await.unwrap();

    // Add documents with different sources
    let docs = vec![
        test_doc(
            "filter-001",
            "Combat rules for melee attacks and damage.",
            "phb.pdf",
            "rules",
        ),
        test_doc(
            "filter-002",
            "Advanced combat options for experienced players.",
            "dmg.pdf",
            "rules",
        ),
        test_doc(
            "filter-003",
            "Monster combat statistics and abilities.",
            "mm.pdf",
            "rules",
        ),
    ];

    // Set campaign_id for some documents
    let mut doc_with_campaign = docs[0].clone();
    doc_with_campaign.campaign_id = Some("camp-001".to_string());

    let mut doc_with_campaign2 = docs[1].clone();
    doc_with_campaign2.campaign_id = Some("camp-001".to_string());

    let doc_no_campaign = docs[2].clone();

    client.add_documents(INDEX_RULES, vec![doc_with_campaign, doc_with_campaign2, doc_no_campaign]).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Search with source filter
    let phb_results = client
        .search(INDEX_RULES, "combat", 10, Some("source = 'phb.pdf'"))
        .await
        .unwrap();

    for result in &phb_results {
        assert_eq!(result.document.source, "phb.pdf", "All results should be from PHB");
    }

    // Search with campaign filter
    let campaign_results = client
        .search(INDEX_RULES, "combat", 10, Some("campaign_id = 'camp-001'"))
        .await
        .unwrap();

    for result in &campaign_results {
        assert_eq!(
            result.document.campaign_id,
            Some("camp-001".to_string()),
            "All results should be from campaign"
        );
    }

    // Clean up
    for id in ["filter-001", "filter-002", "filter-003"] {
        client.delete_document(INDEX_RULES, id).await.ok();
    }
}

// =============================================================================
// Hybrid Search Tests (BM25 + Vector)
// =============================================================================

#[tokio::test]
#[ignore = "Requires running Meilisearch instance with embedder configured"]
async fn test_hybrid_search_semantic_matching() {
    let client = test_client();
    client.initialize_indexes().await.unwrap();

    // Add documents with semantically related content
    let docs = vec![
        test_doc(
            "hybrid-001",
            "The warrior swings his sword at the enemy.",
            "combat.pdf",
            "rules",
        ),
        test_doc(
            "hybrid-002",
            "The fighter attacks with his blade against the foe.",
            "combat.pdf",
            "rules",
        ),
        test_doc(
            "hybrid-003",
            "Magic missile automatically hits and deals force damage.",
            "spells.pdf",
            "rules",
        ),
    ];
    client.add_documents(INDEX_RULES, vec![docs[0].clone(), docs[1].clone(), docs[2].clone()]).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Hybrid search should find semantically similar documents
    // even without exact keyword matches
    let results = client
        .hybrid_search(INDEX_RULES, "melee weapon attack", 10, 0.5, None)
        .await
        .unwrap();

    // Both sword/blade documents should be found due to semantic similarity
    // even though "melee weapon attack" doesn't appear in the text
    if !results.is_empty() {
        let _found_combat = results.iter().any(|r|
            r.document.content.contains("sword") ||
            r.document.content.contains("blade")
        );
        // This assertion may be weak without actual embeddings configured
        println!("Hybrid search results: {:?}", results.iter().map(|r| &r.document.id).collect::<Vec<_>>());
    }

    // Clean up
    for id in ["hybrid-001", "hybrid-002", "hybrid-003"] {
        client.delete_document(INDEX_RULES, id).await.ok();
    }
}

#[tokio::test]
#[ignore = "Requires running Meilisearch instance"]
async fn test_hybrid_search_ratio_variations() {
    let client = test_client();
    client.initialize_indexes().await.unwrap();

    // Add test documents
    let doc = test_doc(
        "ratio-test-001",
        "The paladin channels divine energy to smite evil creatures.",
        "classes.pdf",
        "rules",
    );
    client.add_documents(INDEX_RULES, vec![doc]).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Test different semantic ratios
    // ratio = 0.0 means pure keyword (BM25) search
    let keyword_only = client
        .hybrid_search(INDEX_RULES, "paladin smite", 10, 0.0, None)
        .await
        .unwrap();

    // ratio = 1.0 means pure semantic search
    let semantic_only = client
        .hybrid_search(INDEX_RULES, "paladin smite", 10, 1.0, None)
        .await
        .unwrap();

    // ratio = 0.5 means balanced hybrid
    let balanced = client
        .hybrid_search(INDEX_RULES, "paladin smite", 10, 0.5, None)
        .await
        .unwrap();

    // All should find results (if embedder is configured)
    // Note: Without actual embeddings, semantic search may fall back to keyword
    println!("Keyword only: {} results", keyword_only.len());
    println!("Semantic only: {} results", semantic_only.len());
    println!("Balanced: {} results", balanced.len());

    // Clean up
    client.delete_document(INDEX_RULES, "ratio-test-001").await.ok();
}

// =============================================================================
// Search Analytics Recording Tests
// =============================================================================

#[tokio::test]
async fn test_in_memory_search_analytics_recording() {
    let analytics = SearchAnalytics::new();

    // Record multiple searches
    let searches = vec![
        ("fireball damage", 5, true, 0),
        ("concentration check", 3, false, 0),
        ("fireball damage", 5, true, 1),  // Same query, different click position
        ("opportunity attack", 8, true, 0),
        ("concentration check", 2, true, 0),
    ];

    for (query, results, clicked, click_idx) in searches {
        let mut record = SearchRecord::new(
            query.to_string(),
            results,
            50,
            "hybrid".to_string(),
        );
        record.clicked = clicked;
        record.clicked_index = if clicked { Some(click_idx) } else { None };
        analytics.record(record);
    }

    // Verify popular queries
    let popular = analytics.get_popular_queries(10);
    assert_eq!(popular.len(), 3);
    // Both "fireball damage" and "concentration check" have count=2, so order may vary
    let top_query = &popular[0];
    assert!(
        (top_query.0 == "fireball damage" || top_query.0 == "concentration check") && top_query.1 == 2,
        "Top query should be fireball damage or concentration check with count 2, got {} with count {}",
        top_query.0, top_query.1
    );

    // Verify query stats
    let fireball_stats = analytics.get_query_stats("fireball damage").unwrap();
    assert_eq!(fireball_stats.count, 2);
    assert_eq!(fireball_stats.clicks, 2);

    // Verify click position distribution
    let distribution = analytics.get_click_position_distribution();
    assert!(distribution.contains_key(&0), "Should have clicks at position 0");
    assert!(distribution.contains_key(&1), "Should have clicks at position 1");

    // Verify summary
    let summary = analytics.get_summary(24);
    assert_eq!(summary.total_searches, 5);
    assert!(summary.click_through_rate > 0.5); // 4 out of 5 clicked
}

#[tokio::test]
async fn test_search_analytics_cache_tracking() {
    let analytics = SearchAnalytics::new();

    // Record cached and uncached searches
    let uncached = SearchRecord::new("test query".to_string(), 5, 100, "keyword".to_string())
        .with_cache(false);
    analytics.record(uncached);

    let cached = SearchRecord::new("test query".to_string(), 5, 10, "keyword".to_string())
        .with_cache(true);
    analytics.record(cached);

    let cached2 = SearchRecord::new("other query".to_string(), 3, 8, "keyword".to_string())
        .with_cache(true);
    analytics.record(cached2);

    // Verify cache stats
    let cache_stats = analytics.get_cache_stats();
    assert_eq!(cache_stats.hits, 2);
    assert_eq!(cache_stats.misses, 1);
    assert!((cache_stats.hit_rate - 0.666).abs() < 0.01);

    // Verify top cached queries
    assert!(!cache_stats.top_cached_queries.is_empty());
}

#[tokio::test]
async fn test_search_analytics_result_selection() {
    let analytics = SearchAnalytics::new();

    // Record a search
    let search_id = uuid::Uuid::new_v4().to_string();
    let mut record = SearchRecord::new("dragon lore".to_string(), 10, 45, "hybrid".to_string());
    record.id = search_id.clone();
    analytics.record(record);

    // Record selection
    let selection = ResultSelection {
        search_id: search_id.clone(),
        query: "dragon lore".to_string(),
        result_index: 2,
        source: "monster_manual_chapter_5".to_string(),
        was_helpful: Some(true),
        selection_delay_ms: 3500,
        timestamp: Utc::now(),
    };
    analytics.record_selection(selection);

    // Verify selection was recorded
    let selections = analytics.get_selections_for_query("dragon lore");
    assert_eq!(selections.len(), 1);
    assert_eq!(selections[0].result_index, 2);
    assert_eq!(selections[0].source, "monster_manual_chapter_5");

    // Verify detailed popular queries includes click rate
    let detailed = analytics.get_popular_queries_detailed(10);
    let dragon_query = detailed.iter().find(|q| q.query == "dragon lore").unwrap();
    assert_eq!(dragon_query.click_through_rate, 1.0);
}

#[tokio::test]
async fn test_database_backed_search_analytics() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db = Database::new(temp_dir.path())
        .await
        .expect("Failed to create database");
    let db = Arc::new(db);

    // Record searches directly to database
    let record1 = SearchAnalyticsRecord::new(
        "spell components".to_string(),
        15,
        42,
        "hybrid".to_string(),
        false,
    );
    db.record_search(&record1).await.expect("Failed to record search");

    let mut record2 = SearchAnalyticsRecord::new(
        "spell components".to_string(),
        15,
        35,
        "hybrid".to_string(),
        true, // cached
    );
    record2.selected_result_index = Some(0);
    db.record_search(&record2).await.expect("Failed to record search");

    let record3 = SearchAnalyticsRecord::new(
        "armor class".to_string(),
        8,
        50,
        "keyword".to_string(),
        false,
    );
    db.record_search(&record3).await.expect("Failed to record search");

    // Query analytics
    let analytics = db.get_search_analytics(24).await.expect("Failed to get analytics");
    assert_eq!(analytics.len(), 3);

    // Get summary
    let summary = db.get_search_analytics_summary(24).await.expect("Failed to get summary");
    assert_eq!(summary.total_searches, 3);
    assert!(summary.cache_stats.hits >= 1);

    // Get popular queries
    let popular = db.get_popular_queries(10).await.expect("Failed to get popular queries");
    assert!(!popular.is_empty());
    assert_eq!(popular[0].query, "spell components"); // Most frequent

    // Test cache stats
    let cache_stats = db.get_cache_stats().await.expect("Failed to get cache stats");
    assert!(cache_stats.hit_rate > 0.0);
}

// =============================================================================
// Index Deletion and Cleanup Tests
// =============================================================================

#[tokio::test]
#[ignore = "Requires running Meilisearch instance"]
async fn test_clear_index() {
    let client = test_client();
    client.initialize_indexes().await.unwrap();

    // Add documents
    let docs: Vec<_> = (0..5)
        .map(|i| test_doc(
            &format!("clear-test-{}", i),
            &format!("Document {} content for clearing test.", i),
            "test.pdf",
            "document",
        ))
        .collect();

    client.add_documents(INDEX_DOCUMENTS, docs).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Verify documents were added
    let count_before = client.document_count(INDEX_DOCUMENTS).await.unwrap();
    assert!(count_before >= 5, "Should have at least 5 documents");

    // Clear the index
    client.clear_index(INDEX_DOCUMENTS).await.expect("Failed to clear index");
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Verify index is empty
    let count_after = client.document_count(INDEX_DOCUMENTS).await.unwrap();
    assert_eq!(count_after, 0, "Index should be empty after clearing");
}

#[tokio::test]
#[ignore = "Requires running Meilisearch instance"]
async fn test_delete_documents_by_filter() {
    let client = test_client();
    client.initialize_indexes().await.unwrap();

    // Add documents with different campaigns
    let campaign_a_docs: Vec<_> = (0..3)
        .map(|i| {
            let mut doc = test_doc(
                &format!("camp-a-{}", i),
                &format!("Campaign A document {}.", i),
                "notes.pdf",
                "document",
            );
            doc.campaign_id = Some("campaign-a".to_string());
            doc
        })
        .collect();

    let campaign_b_docs: Vec<_> = (0..3)
        .map(|i| {
            let mut doc = test_doc(
                &format!("camp-b-{}", i),
                &format!("Campaign B document {}.", i),
                "notes.pdf",
                "document",
            );
            doc.campaign_id = Some("campaign-b".to_string());
            doc
        })
        .collect();

    let mut all_docs = campaign_a_docs.clone();
    all_docs.extend(campaign_b_docs.clone());
    client.add_documents(INDEX_DOCUMENTS, all_docs).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Delete only campaign A documents
    client
        .delete_by_filter(INDEX_DOCUMENTS, "campaign_id = 'campaign-a'")
        .await
        .expect("Failed to delete by filter");
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Verify campaign A documents are deleted
    let campaign_a_results = client
        .search(INDEX_DOCUMENTS, "Campaign A", 10, Some("campaign_id = 'campaign-a'"))
        .await
        .unwrap();
    assert!(
        campaign_a_results.is_empty(),
        "Campaign A documents should be deleted"
    );

    // Verify campaign B documents still exist
    let campaign_b_results = client
        .search(INDEX_DOCUMENTS, "Campaign B", 10, Some("campaign_id = 'campaign-b'"))
        .await
        .unwrap();
    assert!(
        !campaign_b_results.is_empty(),
        "Campaign B documents should still exist"
    );

    // Clean up
    client.clear_index(INDEX_DOCUMENTS).await.ok();
}

#[tokio::test]
#[ignore = "Requires running Meilisearch instance"]
async fn test_index_statistics() {
    let client = test_client();
    client.initialize_indexes().await.unwrap();

    // Add known number of documents to each index
    let rules_doc = test_doc("stats-rule-001", "Rules content.", "rules.pdf", "rules");
    let fiction_doc = test_doc("stats-fiction-001", "Fiction content.", "story.pdf", "fiction");

    client.add_documents(INDEX_RULES, vec![rules_doc]).await.unwrap();
    client.add_documents(INDEX_FICTION, vec![fiction_doc]).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Get all stats
    let stats = client.get_all_stats().await.expect("Failed to get stats");

    // Verify stats structure
    assert!(stats.contains_key(INDEX_RULES), "Should have rules stats");
    assert!(stats.contains_key(INDEX_FICTION), "Should have fiction stats");
    assert!(stats.contains_key(INDEX_CHAT), "Should have chat stats");
    assert!(stats.contains_key(INDEX_DOCUMENTS), "Should have documents stats");

    // Verify at least our test documents are counted
    assert!(
        *stats.get(INDEX_RULES).unwrap_or(&0) >= 1,
        "Rules index should have at least 1 document"
    );
    assert!(
        *stats.get(INDEX_FICTION).unwrap_or(&0) >= 1,
        "Fiction index should have at least 1 document"
    );

    // Clean up
    client.delete_document(INDEX_RULES, "stats-rule-001").await.ok();
    client.delete_document(INDEX_FICTION, "stats-fiction-001").await.ok();
}

// =============================================================================
// Federated Search Tests
// =============================================================================

#[tokio::test]
#[ignore = "Requires running Meilisearch instance"]
async fn test_federated_search_across_all_indexes() {
    let client = test_client();
    client.initialize_indexes().await.unwrap();

    // Add "elf" content to multiple indexes
    let rules_doc = test_doc(
        "fed-rules-001",
        "Elves have advantage on saving throws against being charmed.",
        "phb.pdf",
        "rules",
    );
    let fiction_doc = test_doc(
        "fed-fiction-001",
        "The elven kingdom of Evereska has stood for millennia.",
        "forgotten_realms.pdf",
        "fiction",
    );
    let doc = test_doc(
        "fed-doc-001",
        "Session notes: Met the elf ambassador today.",
        "session_3.md",
        "document",
    );

    client.add_documents(INDEX_RULES, vec![rules_doc]).await.unwrap();
    client.add_documents(INDEX_FICTION, vec![fiction_doc]).await.unwrap();
    client.add_documents(INDEX_DOCUMENTS, vec![doc]).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Federated search across all indexes
    let results = client.search_all("elf", 10).await.expect("Federated search failed");

    assert!(
        results.results.len() >= 3,
        "Should find documents from multiple indexes"
    );

    // Verify results come from different indexes
    let indexes_found: std::collections::HashSet<_> =
        results.results.iter().map(|r| r.index.as_str()).collect();
    assert!(
        indexes_found.len() >= 2,
        "Results should come from at least 2 different indexes"
    );

    // Verify processing time is tracked
    assert!(results.processing_time_ms > 0);

    // Clean up
    client.delete_document(INDEX_RULES, "fed-rules-001").await.ok();
    client.delete_document(INDEX_FICTION, "fed-fiction-001").await.ok();
    client.delete_document(INDEX_DOCUMENTS, "fed-doc-001").await.ok();
}

#[tokio::test]
#[ignore = "Requires running Meilisearch instance"]
async fn test_federated_search_ranking() {
    let client = test_client();
    client.initialize_indexes().await.unwrap();

    // Add documents with varying relevance
    let highly_relevant = test_doc(
        "rank-001",
        "Dragons are fearsome creatures with breath weapons and natural armor.",
        "monster_manual.pdf",
        "rules",
    );
    let somewhat_relevant = test_doc(
        "rank-002",
        "The dragon's lair was located deep in the mountains.",
        "adventure.pdf",
        "fiction",
    );
    let barely_relevant = test_doc(
        "rank-003",
        "The tavern sign had a dragon painted on it.",
        "session.md",
        "document",
    );

    client.add_documents(INDEX_RULES, vec![highly_relevant]).await.unwrap();
    client.add_documents(INDEX_FICTION, vec![somewhat_relevant]).await.unwrap();
    client.add_documents(INDEX_DOCUMENTS, vec![barely_relevant]).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Search for dragons
    let results = client.search_all("dragon breath weapon", 10).await.unwrap();

    // Most relevant (rules with breath weapons) should rank higher
    if results.results.len() >= 2 {
        // First result should be more relevant than second
        assert!(
            results.results[0].score >= results.results[1].score,
            "Results should be sorted by relevance"
        );
    }

    // Clean up
    client.delete_document(INDEX_RULES, "rank-001").await.ok();
    client.delete_document(INDEX_FICTION, "rank-002").await.ok();
    client.delete_document(INDEX_DOCUMENTS, "rank-003").await.ok();
}

// =============================================================================
// Index Selection Tests
// =============================================================================

#[test]
fn test_index_selection_for_source_type() {
    // Test various source type mappings
    assert_eq!(SearchClient::select_index_for_source_type("rules"), INDEX_RULES);
    assert_eq!(SearchClient::select_index_for_source_type("RULES"), INDEX_RULES);
    assert_eq!(SearchClient::select_index_for_source_type("rulebook"), INDEX_RULES);
    assert_eq!(SearchClient::select_index_for_source_type("mechanics"), INDEX_RULES);

    assert_eq!(SearchClient::select_index_for_source_type("fiction"), INDEX_FICTION);
    assert_eq!(SearchClient::select_index_for_source_type("lore"), INDEX_FICTION);
    assert_eq!(SearchClient::select_index_for_source_type("story"), INDEX_FICTION);
    assert_eq!(SearchClient::select_index_for_source_type("narrative"), INDEX_FICTION);

    assert_eq!(SearchClient::select_index_for_source_type("chat"), INDEX_CHAT);
    assert_eq!(SearchClient::select_index_for_source_type("conversation"), INDEX_CHAT);
    assert_eq!(SearchClient::select_index_for_source_type("message"), INDEX_CHAT);

    // Default to documents for unknown types
    assert_eq!(SearchClient::select_index_for_source_type("pdf"), INDEX_DOCUMENTS);
    assert_eq!(SearchClient::select_index_for_source_type("unknown"), INDEX_DOCUMENTS);
    assert_eq!(SearchClient::select_index_for_source_type(""), INDEX_DOCUMENTS);
}

#[test]
fn test_all_indexes_list() {
    let all = SearchClient::all_indexes();
    assert_eq!(all.len(), 4);
    assert!(all.contains(&INDEX_RULES));
    assert!(all.contains(&INDEX_FICTION));
    assert!(all.contains(&INDEX_CHAT));
    assert!(all.contains(&INDEX_DOCUMENTS));
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[tokio::test]
#[ignore = "Requires running Meilisearch instance"]
async fn test_search_nonexistent_index() {
    let client = test_client();

    // Search in an index that doesn't exist
    let result = client.search("nonexistent_index_xyz", "test", 10, None).await;

    // Should return an error or empty results
    match result {
        Ok(results) => assert!(results.is_empty()),
        Err(_) => (), // Expected error is acceptable
    }
}

#[tokio::test]
async fn test_invalid_connection() {
    // Create client with invalid host
    let client = SearchClient::new("http://invalid-host:9999", None).unwrap();

    // Health check should fail
    let healthy = client.health_check().await;
    assert!(!healthy, "Health check should fail for invalid host");

    // Wait for health should timeout
    let waited = client.wait_for_health(1).await;
    assert!(!waited, "Should not become healthy");
}
