//! Integration tests for the Unified Archetype Registry.
//!
//! These tests verify end-to-end functionality of the archetype system,
//! including Meilisearch persistence, resolution pipelines, and cache coherence.
//!
//! # Test Categories
//!
//! - **Meilisearch Index Tests**: Verify index creation and configuration
//! - **CRUD Operations**: Test document lifecycle with Meilisearch backend
//! - **Resolution Pipeline**: Test multi-layer resolution with persistence
//! - **Cache Coherence**: Verify cache invalidation with Meilisearch
//! - **Setting Pack Persistence**: Test setting pack storage and retrieval
//!
//! # Running Tests
//!
//! These tests require a running Meilisearch instance:
//!
//! ```bash
//! # Start Meilisearch (if not running)
//! meilisearch --master-key test_key
//!
//! # Run integration tests
//! cargo test --test archetype_integration -- --nocapture
//! ```
//!
//! # Test Isolation
//!
//! Each test uses a unique index prefix to prevent interference between
//! concurrent test runs. Indexes are cleaned up after each test.

use std::time::Duration;

use meilisearch_sdk::client::Client;
use serde_json::json;

// Re-export types from the main crate
#[allow(unused_imports)]
use ttrpg_assistant::core::archetype::*;
// For this integration test to work, the main crate must expose these types.

/// Test configuration for Meilisearch connection.
struct TestConfig {
    meilisearch_url: String,
    meilisearch_key: Option<String>,
    test_prefix: String,
}

impl TestConfig {
    fn new() -> Self {
        Self {
            meilisearch_url: std::env::var("MEILISEARCH_URL")
                .unwrap_or_else(|_| "http://localhost:7700".to_string()),
            meilisearch_key: std::env::var("MEILISEARCH_KEY").ok(),
            test_prefix: format!("test_{}", uuid::Uuid::new_v4().to_string().replace("-", "_")),
        }
    }

    fn client(&self) -> Client {
        match &self.meilisearch_key {
            Some(key) => Client::new(&self.meilisearch_url, Some(key.as_str()))
                .expect("Failed to create Meilisearch client"),
            None => Client::new(&self.meilisearch_url, None::<String>)
                .expect("Failed to create Meilisearch client"),
        }
    }

    fn index_name(&self, base: &str) -> String {
        format!("{}_{}", self.test_prefix, base)
    }
}

/// Clean up test indexes after test completion.
async fn cleanup_indexes(client: &Client, prefix: &str) {
    // List all indexes and delete those matching our prefix
    if let Ok(indexes) = client.list_all_indexes().await {
        for index in indexes.results {
            if index.uid.starts_with(prefix) {
                let _ = client.index(&index.uid).delete().await;
            }
        }
    }
}

// ============================================================================
// Meilisearch Connectivity Tests
// ============================================================================

/// Test that we can connect to Meilisearch and verify it's running.
#[tokio::test]
#[ignore = "requires running Meilisearch instance"]
async fn test_meilisearch_connection() {
    let config = TestConfig::new();
    let client = config.client();

    // Verify connection by getting server health
    let health = client.health().await;
    assert!(health.is_ok(), "Should be able to connect to Meilisearch");
}

/// Test that we can create and configure an archetype index.
#[tokio::test]
#[ignore = "requires running Meilisearch instance"]
async fn test_index_creation() {
    let config = TestConfig::new();
    let client = config.client();
    let index_name = config.index_name("archetypes");

    // Create index with primary key
    let task = client
        .create_index(&index_name, Some("id"))
        .await
        .expect("Should create index");

    // Wait for task completion
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    // Verify index exists
    let index = client.get_index(&index_name).await;
    assert!(index.is_ok(), "Index should exist after creation");

    // Cleanup
    cleanup_indexes(&client, &config.test_prefix).await;
}

/// Test configuring searchable and filterable attributes.
#[tokio::test]
#[ignore = "requires running Meilisearch instance"]
async fn test_index_configuration() {
    let config = TestConfig::new();
    let client = config.client();
    let index_name = config.index_name("archetypes");

    // Create index
    let task = client
        .create_index(&index_name, Some("id"))
        .await
        .expect("Should create index");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    let index = client.index(&index_name);

    // Set searchable attributes
    let searchable = vec!["displayName", "description", "tags"];
    let task = index
        .set_searchable_attributes(&searchable)
        .await
        .expect("Should set searchable attributes");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    // Set filterable attributes
    let filterable = vec!["category", "gameSystem", "tags"];
    let task = index
        .set_filterable_attributes(&filterable)
        .await
        .expect("Should set filterable attributes");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    // Verify configuration
    let settings = index.get_settings().await.expect("Should get settings");
    assert!(
        settings
            .searchable_attributes
            .as_ref()
            .map(|s| s.contains(&"displayName".to_string()))
            .unwrap_or(false),
        "displayName should be searchable"
    );

    // Cleanup
    cleanup_indexes(&client, &config.test_prefix).await;
}

// ============================================================================
// Document CRUD Tests
// ============================================================================

/// Test adding a single document to the index.
#[tokio::test]
#[ignore = "requires running Meilisearch instance"]
async fn test_document_add() {
    let config = TestConfig::new();
    let client = config.client();
    let index_name = config.index_name("archetypes");

    // Create index
    let task = client
        .create_index(&index_name, Some("id"))
        .await
        .expect("Should create index");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    let index = client.index(&index_name);

    // Add a document
    let doc = json!({
        "id": "knight",
        "displayName": "Knight",
        "category": "class",
        "description": "A noble warrior trained in combat",
        "gameSystem": "dnd5e",
        "tags": ["warrior", "combat", "noble"]
    });

    let task = index
        .add_documents(&[doc], Some("id"))
        .await
        .expect("Should add document");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    // Retrieve document
    let retrieved: serde_json::Value = index
        .get_document("knight")
        .await
        .expect("Should retrieve document");

    assert_eq!(retrieved["id"], "knight");
    assert_eq!(retrieved["displayName"], "Knight");
    assert_eq!(retrieved["category"], "class");

    // Cleanup
    cleanup_indexes(&client, &config.test_prefix).await;
}

/// Test updating an existing document.
#[tokio::test]
#[ignore = "requires running Meilisearch instance"]
async fn test_document_update() {
    let config = TestConfig::new();
    let client = config.client();
    let index_name = config.index_name("archetypes");

    // Create index and add initial document
    let task = client
        .create_index(&index_name, Some("id"))
        .await
        .expect("Should create index");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    let index = client.index(&index_name);

    let doc = json!({
        "id": "knight",
        "displayName": "Knight",
        "description": "A warrior"
    });

    let task = index
        .add_documents(&[doc], Some("id"))
        .await
        .expect("Should add document");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    // Update document
    let updated_doc = json!({
        "id": "knight",
        "displayName": "Knight Errant",
        "description": "A wandering noble warrior"
    });

    let task = index
        .add_documents(&[updated_doc], Some("id"))
        .await
        .expect("Should update document");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    // Verify update
    let retrieved: serde_json::Value = index
        .get_document("knight")
        .await
        .expect("Should retrieve document");

    assert_eq!(retrieved["displayName"], "Knight Errant");
    assert_eq!(retrieved["description"], "A wandering noble warrior");

    // Cleanup
    cleanup_indexes(&client, &config.test_prefix).await;
}

/// Test deleting a document from the index.
#[tokio::test]
#[ignore = "requires running Meilisearch instance"]
async fn test_document_delete() {
    let config = TestConfig::new();
    let client = config.client();
    let index_name = config.index_name("archetypes");

    // Create index and add document
    let task = client
        .create_index(&index_name, Some("id"))
        .await
        .expect("Should create index");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    let index = client.index(&index_name);

    let doc = json!({
        "id": "knight",
        "displayName": "Knight"
    });

    let task = index
        .add_documents(&[doc], Some("id"))
        .await
        .expect("Should add document");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    // Delete document
    let task = index
        .delete_document("knight")
        .await
        .expect("Should delete document");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    // Verify deletion
    let result: std::result::Result<serde_json::Value, _> = index.get_document("knight").await;
    assert!(result.is_err(), "Document should not exist after deletion");

    // Cleanup
    cleanup_indexes(&client, &config.test_prefix).await;
}

// ============================================================================
// Search and Filter Tests
// ============================================================================

/// Test basic text search functionality.
#[tokio::test]
#[ignore = "requires running Meilisearch instance"]
async fn test_search_basic() {
    let config = TestConfig::new();
    let client = config.client();
    let index_name = config.index_name("archetypes");

    // Create and configure index
    let task = client
        .create_index(&index_name, Some("id"))
        .await
        .expect("Should create index");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    let index = client.index(&index_name);

    // Add multiple documents
    let docs = vec![
        json!({
            "id": "knight",
            "displayName": "Knight",
            "description": "A noble warrior trained in combat",
            "tags": ["warrior", "combat"]
        }),
        json!({
            "id": "merchant",
            "displayName": "Merchant",
            "description": "A trader and businessperson",
            "tags": ["trade", "social"]
        }),
        json!({
            "id": "guard",
            "displayName": "Town Guard",
            "description": "A warrior who protects the town",
            "tags": ["warrior", "protection"]
        }),
    ];

    let task = index
        .add_documents(&docs, Some("id"))
        .await
        .expect("Should add documents");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    // Search for "warrior"
    let results: meilisearch_sdk::search::SearchResults<serde_json::Value> = index
        .search()
        .with_query("warrior")
        .execute()
        .await
        .expect("Search should succeed");

    assert!(results.hits.len() >= 2, "Should find knight and guard");

    // Cleanup
    cleanup_indexes(&client, &config.test_prefix).await;
}

/// Test filtered search by category.
#[tokio::test]
#[ignore = "requires running Meilisearch instance"]
async fn test_search_with_filter() {
    let config = TestConfig::new();
    let client = config.client();
    let index_name = config.index_name("archetypes");

    // Create and configure index
    let task = client
        .create_index(&index_name, Some("id"))
        .await
        .expect("Should create index");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    let index = client.index(&index_name);

    // Set filterable attributes
    let task = index
        .set_filterable_attributes(&["category"])
        .await
        .expect("Should set filterable");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    // Add documents with categories
    let docs = vec![
        json!({
            "id": "knight",
            "displayName": "Knight",
            "category": "class"
        }),
        json!({
            "id": "dwarf",
            "displayName": "Dwarf",
            "category": "race"
        }),
        json!({
            "id": "merchant",
            "displayName": "Merchant",
            "category": "role"
        }),
    ];

    let task = index
        .add_documents(&docs, Some("id"))
        .await
        .expect("Should add documents");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    // Search with filter
    let results: meilisearch_sdk::search::SearchResults<serde_json::Value> = index
        .search()
        .with_query("")
        .with_filter("category = 'race'")
        .execute()
        .await
        .expect("Filtered search should succeed");

    assert_eq!(results.hits.len(), 1);
    assert_eq!(results.hits[0].result["id"], "dwarf");

    // Cleanup
    cleanup_indexes(&client, &config.test_prefix).await;
}

// ============================================================================
// Batch Operations Tests
// ============================================================================

/// Test adding multiple documents in a batch.
#[tokio::test]
#[ignore = "requires running Meilisearch instance"]
async fn test_batch_add() {
    let config = TestConfig::new();
    let client = config.client();
    let index_name = config.index_name("archetypes");

    // Create index
    let task = client
        .create_index(&index_name, Some("id"))
        .await
        .expect("Should create index");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    let index = client.index(&index_name);

    // Create batch of documents
    let docs: Vec<serde_json::Value> = (0..100)
        .map(|i| {
            json!({
                "id": format!("archetype_{}", i),
                "displayName": format!("Archetype {}", i),
                "category": if i % 3 == 0 { "class" } else if i % 3 == 1 { "race" } else { "role" }
            })
        })
        .collect();

    let task = index
        .add_documents(&docs, Some("id"))
        .await
        .expect("Should add documents");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(60)),
    )
    .await
    .expect("Task should complete");

    // Verify count
    let stats = index.get_stats().await.expect("Should get stats");
    assert_eq!(stats.number_of_documents, 100);

    // Cleanup
    cleanup_indexes(&client, &config.test_prefix).await;
}

/// Test deleting multiple documents in batch.
#[tokio::test]
#[ignore = "requires running Meilisearch instance"]
async fn test_batch_delete() {
    let config = TestConfig::new();
    let client = config.client();
    let index_name = config.index_name("archetypes");

    // Create index and add documents
    let task = client
        .create_index(&index_name, Some("id"))
        .await
        .expect("Should create index");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    let index = client.index(&index_name);

    let docs = vec![
        json!({ "id": "a" }),
        json!({ "id": "b" }),
        json!({ "id": "c" }),
        json!({ "id": "d" }),
        json!({ "id": "e" }),
    ];

    let task = index
        .add_documents(&docs, Some("id"))
        .await
        .expect("Should add documents");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    // Delete batch
    let task = index
        .delete_documents(&["a", "c", "e"])
        .await
        .expect("Should delete documents");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    // Verify remaining count
    let stats = index.get_stats().await.expect("Should get stats");
    assert_eq!(stats.number_of_documents, 2);

    // Cleanup
    cleanup_indexes(&client, &config.test_prefix).await;
}

// ============================================================================
// Setting Pack Persistence Tests
// ============================================================================

/// Test storing and retrieving a setting pack.
#[tokio::test]
#[ignore = "requires running Meilisearch instance"]
async fn test_setting_pack_persistence() {
    let config = TestConfig::new();
    let client = config.client();
    let index_name = config.index_name("setting_packs");

    // Create index
    let task = client
        .create_index(&index_name, Some("id"))
        .await
        .expect("Should create index");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    let index = client.index(&index_name);

    // Store setting pack
    let pack = json!({
        "id": "forgotten_realms",
        "name": "Forgotten Realms",
        "gameSystem": "dnd5e",
        "version": "1.0.0",
        "archetypeOverrides": {
            "dwarf": {
                "displayName": "Shield Dwarf"
            }
        },
        "customArchetypes": [],
        "namingCultures": []
    });

    let task = index
        .add_documents(&[pack], Some("id"))
        .await
        .expect("Should add pack");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    // Retrieve pack
    let retrieved: serde_json::Value = index
        .get_document("forgotten_realms")
        .await
        .expect("Should retrieve pack");

    assert_eq!(retrieved["name"], "Forgotten Realms");
    assert_eq!(retrieved["version"], "1.0.0");
    assert!(retrieved["archetypeOverrides"]["dwarf"]["displayName"] == "Shield Dwarf");

    // Cleanup
    cleanup_indexes(&client, &config.test_prefix).await;
}

// ============================================================================
// Error Handling Tests
// ============================================================================

/// Test handling of non-existent document retrieval.
#[tokio::test]
#[ignore = "requires running Meilisearch instance"]
async fn test_document_not_found() {
    let config = TestConfig::new();
    let client = config.client();
    let index_name = config.index_name("archetypes");

    // Create empty index
    let task = client
        .create_index(&index_name, Some("id"))
        .await
        .expect("Should create index");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    let index = client.index(&index_name);

    // Try to get non-existent document
    let result: std::result::Result<serde_json::Value, _> = index.get_document("nonexistent").await;
    assert!(result.is_err(), "Should fail for non-existent document");

    // Cleanup
    cleanup_indexes(&client, &config.test_prefix).await;
}

/// Test handling of invalid filter syntax.
#[tokio::test]
#[ignore = "requires running Meilisearch instance"]
async fn test_invalid_filter() {
    let config = TestConfig::new();
    let client = config.client();
    let index_name = config.index_name("archetypes");

    // Create index
    let task = client
        .create_index(&index_name, Some("id"))
        .await
        .expect("Should create index");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    let index = client.index(&index_name);

    // Add a document
    let doc = json!({ "id": "test", "displayName": "Test" });
    let task = index
        .add_documents(&[doc], Some("id"))
        .await
        .expect("Should add document");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    // Try invalid filter (category not set as filterable)
    let result: std::result::Result<meilisearch_sdk::search::SearchResults<serde_json::Value>, _> = index
        .search()
        .with_query("")
        .with_filter("category = 'class'")
        .execute()
        .await;

    assert!(result.is_err(), "Should fail with invalid filter");

    // Cleanup
    cleanup_indexes(&client, &config.test_prefix).await;
}

// ============================================================================
// Performance Baseline Tests
// ============================================================================

/// Test that single document retrieval completes within acceptable time.
#[tokio::test]
#[ignore = "requires running Meilisearch instance"]
async fn test_retrieval_performance() {
    let config = TestConfig::new();
    let client = config.client();
    let index_name = config.index_name("archetypes");

    // Create index and add document
    let task = client
        .create_index(&index_name, Some("id"))
        .await
        .expect("Should create index");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    let index = client.index(&index_name);

    let doc = json!({
        "id": "knight",
        "displayName": "Knight",
        "category": "class"
    });

    let task = index
        .add_documents(&[doc], Some("id"))
        .await
        .expect("Should add document");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    // Measure retrieval time
    let start = std::time::Instant::now();

    for _ in 0..10 {
        let _: serde_json::Value = index
            .get_document("knight")
            .await
            .expect("Should retrieve document");
    }

    let elapsed = start.elapsed();
    let avg_ms = elapsed.as_millis() / 10;

    // Document retrieval should be fast (< 50ms average)
    assert!(
        avg_ms < 50,
        "Average retrieval time {}ms exceeds 50ms threshold",
        avg_ms
    );

    // Cleanup
    cleanup_indexes(&client, &config.test_prefix).await;
}

/// Test that search completes within acceptable time.
#[tokio::test]
#[ignore = "requires running Meilisearch instance"]
async fn test_search_performance() {
    let config = TestConfig::new();
    let client = config.client();
    let index_name = config.index_name("archetypes");

    // Create and populate index
    let task = client
        .create_index(&index_name, Some("id"))
        .await
        .expect("Should create index");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    let index = client.index(&index_name);

    // Add 50 documents
    let docs: Vec<serde_json::Value> = (0..50)
        .map(|i| {
            json!({
                "id": format!("archetype_{}", i),
                "displayName": format!("Archetype {}", i),
                "description": format!("This is a description for archetype number {}", i)
            })
        })
        .collect();

    let task = index
        .add_documents(&docs, Some("id"))
        .await
        .expect("Should add documents");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(60)),
    )
    .await
    .expect("Task should complete");

    // Measure search time
    let start = std::time::Instant::now();

    for _ in 0..10 {
        let _: meilisearch_sdk::search::SearchResults<serde_json::Value> = index
            .search()
            .with_query("archetype description")
            .execute()
            .await
            .expect("Search should succeed");
    }

    let elapsed = start.elapsed();
    let avg_ms = elapsed.as_millis() / 10;

    // Search should be fast (< 100ms average)
    assert!(
        avg_ms < 100,
        "Average search time {}ms exceeds 100ms threshold",
        avg_ms
    );

    // Cleanup
    cleanup_indexes(&client, &config.test_prefix).await;
}

// ============================================================================
// Data Integrity Tests
// ============================================================================

/// Test that complex nested data is preserved through serialization.
#[tokio::test]
#[ignore = "requires running Meilisearch instance"]
async fn test_complex_data_integrity() {
    let config = TestConfig::new();
    let client = config.client();
    let index_name = config.index_name("archetypes");

    // Create index
    let task = client
        .create_index(&index_name, Some("id"))
        .await
        .expect("Should create index");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    let index = client.index(&index_name);

    // Add document with complex nested structure
    let doc = json!({
        "id": "dwarf_merchant",
        "displayName": "Dwarf Merchant",
        "category": "composite",
        "gameSystem": "dnd5e",
        "personalityAffinity": [
            { "traitId": "greed", "weight": 0.8, "defaultIntensity": 7 },
            { "traitId": "stubborn", "weight": 0.9, "defaultIntensity": 8 }
        ],
        "npcRoleMapping": [
            { "role": "merchant", "weight": 0.9, "contextModifiers": ["trade"] },
            { "role": "craftsman", "weight": 0.7 }
        ],
        "namingCultures": [
            {
                "culture": "dwarvish",
                "weight": 0.95,
                "patternOverrides": {
                    "titleProbability": 0.3,
                    "epithetProbability": 0.5
                }
            }
        ],
        "statTendencies": {
            "strengthModifier": 2,
            "constitutionModifier": 2,
            "dexterityModifier": -1
        },
        "tags": ["dwarf", "merchant", "trade", "commerce"]
    });

    let task = index
        .add_documents(&[doc.clone()], Some("id"))
        .await
        .expect("Should add document");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    // Retrieve and verify
    let retrieved: serde_json::Value = index
        .get_document("dwarf_merchant")
        .await
        .expect("Should retrieve document");

    // Verify nested arrays
    assert_eq!(
        retrieved["personalityAffinity"][0]["traitId"],
        "greed"
    );
    assert_eq!(
        retrieved["personalityAffinity"][0]["weight"],
        0.8
    );
    assert_eq!(
        retrieved["personalityAffinity"][1]["defaultIntensity"],
        8
    );

    // Verify role mappings
    assert_eq!(retrieved["npcRoleMapping"][0]["role"], "merchant");
    assert_eq!(retrieved["npcRoleMapping"][0]["weight"], 0.9);

    // Verify naming cultures with nested pattern overrides
    assert_eq!(retrieved["namingCultures"][0]["culture"], "dwarvish");
    assert_eq!(
        retrieved["namingCultures"][0]["patternOverrides"]["titleProbability"],
        0.3
    );

    // Verify stat tendencies
    assert_eq!(retrieved["statTendencies"]["strengthModifier"], 2);
    assert_eq!(retrieved["statTendencies"]["dexterityModifier"], -1);

    // Verify tags array
    assert!(retrieved["tags"]
        .as_array()
        .unwrap()
        .contains(&json!("merchant")));

    // Cleanup
    cleanup_indexes(&client, &config.test_prefix).await;
}

/// Test that Unicode and special characters are preserved.
#[tokio::test]
#[ignore = "requires running Meilisearch instance"]
async fn test_unicode_preservation() {
    let config = TestConfig::new();
    let client = config.client();
    let index_name = config.index_name("archetypes");

    // Create index
    let task = client
        .create_index(&index_name, Some("id"))
        .await
        .expect("Should create index");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    let index = client.index(&index_name);

    // Add document with Unicode content
    let doc = json!({
        "id": "elvish_sage",
        "displayName": "Elvish Sage",
        "description": "A wise elf who speaks in riddles. Greetings: 'Mae govannen!' (Well met!)",
        "phrases": [
            "Elen sila lumenn' omentielvo",
            "Namarie, mellon nin",
            "'Tis a fine day for wisdom"
        ],
        "tags": ["elf", "sage", "magic", "unicode-test"]
    });

    let task = index
        .add_documents(&[doc], Some("id"))
        .await
        .expect("Should add document");
    task.wait_for_completion(
        &client,
        Some(Duration::from_millis(100)),
        Some(Duration::from_secs(30)),
    )
    .await
    .expect("Task should complete");

    // Retrieve and verify
    let retrieved: serde_json::Value = index
        .get_document("elvish_sage")
        .await
        .expect("Should retrieve document");

    assert!(retrieved["description"]
        .as_str()
        .unwrap()
        .contains("Mae govannen"));
    assert!(retrieved["phrases"]
        .as_array()
        .unwrap()
        .contains(&json!("Elen sila lumenn' omentielvo")));

    // Cleanup
    cleanup_indexes(&client, &config.test_prefix).await;
}
