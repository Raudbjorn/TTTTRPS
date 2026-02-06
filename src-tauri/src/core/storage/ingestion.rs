//! Document ingestion pipeline for SurrealDB.
//!
//! Handles document chunk storage, embedding association, and library item status updates.
//! This module provides functions for bulk ingestion of document chunks with support for
//! pre-computed embeddings, transactional consistency, and proper error handling.
//!
//! # Tasks Implemented
//!
//! - **3.1.1**: `ingest_chunks()` - Bulk insert document chunks (FR-2.1, FR-6.2)
//! - **3.1.2**: `delete_library_chunks()` - Remove all chunks for a library item (FR-8.2)
//! - **3.1.3**: `ingest_chunks_with_embeddings()` - Batch ingestion with pre-computed embeddings
//!
//! # Example
//!
//! ```no_run
//! use ttrpg_assistant::core::storage::ingestion::{ChunkData, ingest_chunks};
//! use ttrpg_assistant::core::storage::SurrealStorage;
//! use std::path::PathBuf;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let storage = SurrealStorage::new(PathBuf::from("./data")).await?;
//!
//! let chunks = vec![
//!     ChunkData {
//!         content: "The dragon attacks with its fiery breath...".to_string(),
//!         content_type: "rules".to_string(),
//!         page_number: Some(42),
//!         ..Default::default()
//!     },
//! ];
//!
//! let inserted = ingest_chunks(storage.db(), "phb-5e", chunks).await?;
//! println!("Inserted {} chunks", inserted);
//! # Ok(())
//! # }
//! ```

use serde::{Deserialize, Serialize};
use surrealdb::engine::local::Db;
use surrealdb::Surreal;

use super::error::StorageError;

/// Document chunk data for ingestion.
///
/// Represents a single chunk of document content to be stored in SurrealDB.
/// Chunks are linked to a parent `library_item` and can optionally include
/// embeddings for vector search.
///
/// # Fields
///
/// - `content` - The actual text content of the chunk
/// - `content_type` - Category: "rules", "lore", "homebrew", "session_notes"
/// - `page_number` - Single page reference (for simple page tracking)
/// - `page_start`/`page_end` - Page range (for chunks spanning multiple pages)
/// - `section_path` - Hierarchical path like "Chapter 3 > Combat > Actions"
/// - `chapter_title`/`section_title` - Document structure context
/// - `chunk_type` - Semantic type: "table", "stat_block", "spell", "narrative"
/// - `semantic_keywords` - Extracted keywords for hybrid search boosting
/// - `embedding` - 768-dimension vector for semantic search (nomic-embed-text)
/// - `embedding_model` - Model identifier (e.g., "nomic-embed-text-v1.5")
/// - `metadata` - Arbitrary JSON for custom fields
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ChunkData {
    /// The text content of this chunk.
    pub content: String,

    /// Content category: "rules", "lore", "homebrew", "session_notes".
    pub content_type: String,

    /// Single page number reference.
    #[serde(default)]
    pub page_number: Option<i32>,

    /// Starting page for multi-page chunks.
    #[serde(default)]
    pub page_start: Option<i32>,

    /// Ending page for multi-page chunks.
    #[serde(default)]
    pub page_end: Option<i32>,

    /// Hierarchical section path (e.g., "Chapter 3 > Combat > Actions").
    #[serde(default)]
    pub section_path: Option<String>,

    /// Chapter title from document structure.
    #[serde(default)]
    pub chapter_title: Option<String>,

    /// Section title from document structure.
    #[serde(default)]
    pub section_title: Option<String>,

    /// Semantic chunk type: "table", "stat_block", "spell", "narrative".
    #[serde(default)]
    pub chunk_type: Option<String>,

    /// Keywords extracted for hybrid search boosting.
    #[serde(default)]
    pub semantic_keywords: Option<Vec<String>>,

    /// 768-dimension embedding vector for semantic search.
    #[serde(default)]
    pub embedding: Option<Vec<f32>>,

    /// Embedding model identifier (e.g., "nomic-embed-text-v1.5").
    #[serde(default)]
    pub embedding_model: Option<String>,

    /// Arbitrary metadata as JSON.
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

/// Ingest document chunks into SurrealDB.
///
/// Stores chunks in a transaction, linking them to the specified library item.
/// On success, updates the library item status to "ready".
///
/// # Arguments
///
/// * `db` - SurrealDB connection
/// * `library_item_id` - ID of the library item these chunks belong to
/// * `chunks` - Vector of chunk data to ingest
///
/// # Returns
///
/// Number of chunks successfully inserted.
///
/// # Errors
///
/// Returns `StorageError::Transaction` if the transaction fails, or
/// `StorageError::Query` if individual inserts fail.
///
/// # Example
///
/// ```no_run
/// # use ttrpg_assistant::core::storage::ingestion::{ChunkData, ingest_chunks};
/// # use surrealdb::Surreal;
/// # use surrealdb::engine::local::Db;
/// # async fn example(db: &Surreal<Db>) -> Result<(), Box<dyn std::error::Error>> {
/// let chunks = vec![
///     ChunkData {
///         content: "Attack of Opportunity...".to_string(),
///         content_type: "rules".to_string(),
///         page_number: Some(195),
///         chapter_title: Some("Combat".to_string()),
///         ..Default::default()
///     },
/// ];
///
/// let count = ingest_chunks(db, "phb-5e", chunks).await?;
/// # Ok(())
/// # }
/// ```
pub async fn ingest_chunks(
    db: &Surreal<Db>,
    library_item_id: &str,
    chunks: Vec<ChunkData>,
) -> Result<usize, StorageError> {
    if chunks.is_empty() {
        return Ok(0);
    }

    // Start transaction
    db.query("BEGIN TRANSACTION")
        .await
        .map_err(|e| StorageError::Transaction(format!("Failed to begin transaction: {}", e)))?;

    let mut inserted = 0;
    let chunk_count = chunks.len();

    // Convert to owned string for binding (SurrealDB requires 'static lifetimes)
    let library_id_owned = library_item_id.to_string();

    for (i, chunk) in chunks.into_iter().enumerate() {
        let chunk_id = format!("{}-{}", library_item_id, i);

        let query = r#"
            CREATE type::thing('chunk', $chunk_id) CONTENT {
                content: $content,
                library_item: type::thing('library_item', $library_id),
                content_type: $content_type,
                page_number: $page_number,
                page_start: $page_start,
                page_end: $page_end,
                chunk_index: $chunk_index,
                section_path: $section_path,
                chapter_title: $chapter_title,
                section_title: $section_title,
                chunk_type: $chunk_type,
                semantic_keywords: $keywords,
                embedding: $embedding,
                embedding_model: $embedding_model,
                metadata: $metadata
            };
        "#;

        let result = db
            .query(query)
            .bind(("chunk_id", chunk_id))
            .bind(("content", chunk.content))
            .bind(("library_id", library_id_owned.clone()))
            .bind(("content_type", chunk.content_type))
            .bind(("page_number", chunk.page_number))
            .bind(("page_start", chunk.page_start))
            .bind(("page_end", chunk.page_end))
            .bind(("chunk_index", i as i32))
            .bind(("section_path", chunk.section_path))
            .bind(("chapter_title", chunk.chapter_title))
            .bind(("section_title", chunk.section_title))
            .bind(("chunk_type", chunk.chunk_type))
            .bind(("keywords", chunk.semantic_keywords))
            .bind(("embedding", chunk.embedding))
            .bind(("embedding_model", chunk.embedding_model))
            .bind(("metadata", chunk.metadata))
            .await;

        if let Err(e) = result {
            // Rollback on error
            let _ = db.query("CANCEL TRANSACTION").await;
            return Err(StorageError::Query(format!(
                "Failed to insert chunk {} of {}: {}",
                i + 1,
                chunk_count,
                e
            )));
        }

        inserted += 1;
    }

    // Update library item status to "ready"
    let status_result = db
        .query(
            r#"
            UPDATE type::thing('library_item', $id) SET
                status = 'ready',
                updated_at = time::now();
        "#,
        )
        .bind(("id", library_id_owned.clone()))
        .await;

    if let Err(e) = status_result {
        // Rollback on error
        let _ = db.query("CANCEL TRANSACTION").await;
        return Err(StorageError::Query(format!(
            "Failed to update library item status: {}",
            e
        )));
    }

    // Commit transaction
    db.query("COMMIT TRANSACTION")
        .await
        .map_err(|e| StorageError::Transaction(format!("Failed to commit transaction: {}", e)))?;

    tracing::info!(
        library_item_id = %library_item_id,
        chunk_count = inserted,
        "Ingested chunks successfully"
    );

    Ok(inserted)
}

/// Delete all chunks associated with a library item.
///
/// Removes all chunks linked to the specified library item ID.
/// Does not modify the library item itself.
///
/// # Arguments
///
/// * `db` - SurrealDB connection
/// * `library_item_id` - ID of the library item whose chunks should be deleted
///
/// # Returns
///
/// Number of chunks deleted.
///
/// # Errors
///
/// Returns `StorageError::Query` if the delete operation fails.
///
/// # Example
///
/// ```no_run
/// # use ttrpg_assistant::core::storage::ingestion::delete_library_chunks;
/// # use surrealdb::Surreal;
/// # use surrealdb::engine::local::Db;
/// # async fn example(db: &Surreal<Db>) -> Result<(), Box<dyn std::error::Error>> {
/// let deleted = delete_library_chunks(db, "phb-5e").await?;
/// println!("Deleted {} chunks", deleted);
/// # Ok(())
/// # }
/// ```
pub async fn delete_library_chunks(
    db: &Surreal<Db>,
    library_item_id: &str,
) -> Result<usize, StorageError> {
    // First count existing chunks
    let count = get_chunk_count(db, library_item_id).await?;

    if count == 0 {
        return Ok(0);
    }

    // Delete all chunks for this library item
    let library_id_owned = library_item_id.to_string();
    db.query("DELETE chunk WHERE library_item = type::thing('library_item', $id)")
        .bind(("id", library_id_owned))
        .await
        .map_err(|e| StorageError::Query(format!("Failed to delete chunks: {}", e)))?;

    tracing::info!(
        library_item_id = %library_item_id,
        chunk_count = count,
        "Deleted chunks for library item"
    );

    Ok(count)
}

/// Ingest chunks with pre-computed embeddings.
///
/// Convenience function that merges a separate embeddings vector with chunk data
/// before ingestion. Useful when embeddings are generated in a batch process
/// separate from chunk creation.
///
/// # Arguments
///
/// * `db` - SurrealDB connection
/// * `library_item_id` - ID of the library item these chunks belong to
/// * `chunks` - Vector of chunk data to ingest
/// * `embeddings` - Vector of embedding vectors (must match chunks length)
///
/// # Returns
///
/// Number of chunks successfully inserted.
///
/// # Errors
///
/// Returns `StorageError::Config` if chunk and embedding counts don't match,
/// or propagates errors from `ingest_chunks`.
///
/// # Example
///
/// ```no_run
/// # use ttrpg_assistant::core::storage::ingestion::{ChunkData, ingest_chunks_with_embeddings};
/// # use surrealdb::Surreal;
/// # use surrealdb::engine::local::Db;
/// # async fn example(db: &Surreal<Db>) -> Result<(), Box<dyn std::error::Error>> {
/// let chunks = vec![
///     ChunkData {
///         content: "Combat rules...".to_string(),
///         content_type: "rules".to_string(),
///         ..Default::default()
///     },
/// ];
///
/// // Pre-computed embeddings from embedding service
/// let embeddings = vec![vec![0.1f32; 768]];
///
/// let count = ingest_chunks_with_embeddings(db, "phb-5e", chunks, embeddings).await?;
/// # Ok(())
/// # }
/// ```
pub async fn ingest_chunks_with_embeddings(
    db: &Surreal<Db>,
    library_item_id: &str,
    chunks: Vec<ChunkData>,
    embeddings: Vec<Vec<f32>>,
) -> Result<usize, StorageError> {
    if chunks.len() != embeddings.len() {
        return Err(StorageError::Config(format!(
            "Chunk count ({}) must match embedding count ({})",
            chunks.len(),
            embeddings.len()
        )));
    }

    let chunks_with_embeddings: Vec<ChunkData> = chunks
        .into_iter()
        .zip(embeddings)
        .map(|(mut chunk, embedding)| {
            chunk.embedding = Some(embedding);
            chunk
        })
        .collect();

    ingest_chunks(db, library_item_id, chunks_with_embeddings).await
}

/// Get the number of chunks for a library item.
///
/// Counts all chunks linked to the specified library item.
///
/// # Arguments
///
/// * `db` - SurrealDB connection
/// * `library_item_id` - ID of the library item to count chunks for
///
/// # Returns
///
/// Number of chunks associated with the library item.
///
/// # Errors
///
/// Returns `StorageError::Query` if the count query fails.
///
/// # Example
///
/// ```no_run
/// # use ttrpg_assistant::core::storage::ingestion::get_chunk_count;
/// # use surrealdb::Surreal;
/// # use surrealdb::engine::local::Db;
/// # async fn example(db: &Surreal<Db>) -> Result<(), Box<dyn std::error::Error>> {
/// let count = get_chunk_count(db, "phb-5e").await?;
/// println!("Library item has {} chunks", count);
/// # Ok(())
/// # }
/// ```
pub async fn get_chunk_count(db: &Surreal<Db>, library_item_id: &str) -> Result<usize, StorageError> {
    #[derive(Debug, Deserialize)]
    struct CountResult {
        count: i64,
    }

    let library_id_owned = library_item_id.to_string();
    let result: Option<CountResult> = db
        .query(
            "SELECT count() as count FROM chunk WHERE library_item = type::thing('library_item', $id) GROUP ALL",
        )
        .bind(("id", library_id_owned))
        .await
        .map_err(|e| StorageError::Query(format!("Failed to count chunks: {}", e)))?
        .take(0)
        .map_err(|e| StorageError::Query(format!("Failed to extract count: {}", e)))?;

    Ok(result.map(|r| r.count as usize).unwrap_or(0))
}

/// Update embeddings for existing chunks.
///
/// Updates the embedding field for chunks that already exist in the database.
/// Useful for re-embedding with a different model or updating after model changes.
///
/// # Arguments
///
/// * `db` - SurrealDB connection
/// * `chunk_ids` - Vector of chunk IDs to update
/// * `embeddings` - Vector of new embedding vectors (must match chunk_ids length)
/// * `model` - Embedding model identifier
///
/// # Returns
///
/// Number of chunks updated.
///
/// # Errors
///
/// Returns `StorageError::Config` if counts don't match, or
/// `StorageError::Query` if update fails.
pub async fn update_chunk_embeddings(
    db: &Surreal<Db>,
    chunk_ids: Vec<String>,
    embeddings: Vec<Vec<f32>>,
    model: &str,
) -> Result<usize, StorageError> {
    if chunk_ids.len() != embeddings.len() {
        return Err(StorageError::Config(format!(
            "Chunk ID count ({}) must match embedding count ({})",
            chunk_ids.len(),
            embeddings.len()
        )));
    }

    let model_owned = model.to_string();
    let mut updated = 0;

    for (chunk_id, embedding) in chunk_ids.into_iter().zip(embeddings.into_iter()) {
        db.query(
            r#"
            UPDATE type::thing('chunk', $id) SET
                embedding = $embedding,
                embedding_model = $model;
        "#,
        )
        .bind(("id", chunk_id))
        .bind(("embedding", embedding))
        .bind(("model", model_owned.clone()))
        .await
        .map_err(|e| {
            StorageError::Query(format!("Failed to update embedding: {}", e))
        })?;

        updated += 1;
    }

    tracing::debug!(
        updated_count = updated,
        model = %model,
        "Updated chunk embeddings"
    );

    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use surrealdb::engine::local::RocksDb;
    use tempfile::TempDir;

    /// Helper to create a test database with schema
    async fn setup_test_db() -> (Surreal<Db>, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db = Surreal::new::<RocksDb>(temp_dir.path())
            .await
            .expect("Failed to create database");

        db.use_ns("test")
            .use_db("test")
            .await
            .expect("Failed to select namespace");

        // Apply minimal schema for tests
        db.query(
            r#"
            DEFINE TABLE library_item SCHEMAFULL;
            DEFINE FIELD slug ON library_item TYPE string;
            DEFINE FIELD title ON library_item TYPE string;
            DEFINE FIELD status ON library_item TYPE string DEFAULT "pending";
            DEFINE FIELD updated_at ON library_item TYPE option<datetime>;
            DEFINE INDEX library_slug ON library_item FIELDS slug UNIQUE;

            DEFINE TABLE chunk SCHEMAFULL;
            DEFINE FIELD content ON chunk TYPE string;
            DEFINE FIELD library_item ON chunk TYPE record<library_item>;
            DEFINE FIELD content_type ON chunk TYPE string;
            DEFINE FIELD page_number ON chunk TYPE option<int>;
            DEFINE FIELD page_start ON chunk TYPE option<int>;
            DEFINE FIELD page_end ON chunk TYPE option<int>;
            DEFINE FIELD chunk_index ON chunk TYPE option<int>;
            DEFINE FIELD section_path ON chunk TYPE option<string>;
            DEFINE FIELD chapter_title ON chunk TYPE option<string>;
            DEFINE FIELD section_title ON chunk TYPE option<string>;
            DEFINE FIELD chunk_type ON chunk TYPE option<string>;
            DEFINE FIELD semantic_keywords ON chunk TYPE option<array<string>>;
            DEFINE FIELD embedding ON chunk TYPE option<array<float>>;
            DEFINE FIELD embedding_model ON chunk TYPE option<string>;
            DEFINE FIELD metadata ON chunk TYPE option<object>;
            DEFINE INDEX chunk_library ON chunk FIELDS library_item;
        "#,
        )
        .await
        .expect("Failed to apply schema");

        (db, temp_dir)
    }

    /// Helper to create a test library item
    async fn create_test_library_item(db: &Surreal<Db>, id: &str, title: &str) {
        // Convert to owned strings for SurrealDB bind (requires 'static)
        let id_owned = id.to_string();
        let title_owned = title.to_string();

        db.query(
            r#"
            CREATE type::thing('library_item', $id) CONTENT {
                slug: $id,
                title: $title,
                status: 'pending'
            };
        "#,
        )
        .bind(("id", id_owned))
        .bind(("title", title_owned))
        .await
        .expect("Failed to create library item");
    }

    #[tokio::test]
    async fn test_ingest_chunks_basic() {
        let (db, _temp) = setup_test_db().await;
        create_test_library_item(&db, "test-doc", "Test Document").await;

        let chunks = vec![
            ChunkData {
                content: "First chunk content".to_string(),
                content_type: "rules".to_string(),
                page_number: Some(1),
                ..Default::default()
            },
            ChunkData {
                content: "Second chunk content".to_string(),
                content_type: "rules".to_string(),
                page_number: Some(2),
                ..Default::default()
            },
        ];

        let result = ingest_chunks(&db, "test-doc", chunks).await;
        assert!(result.is_ok(), "Ingest failed: {:?}", result.err());
        assert_eq!(result.unwrap(), 2);

        // Verify chunk count
        let count = get_chunk_count(&db, "test-doc").await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_ingest_chunks_empty() {
        let (db, _temp) = setup_test_db().await;
        create_test_library_item(&db, "empty-doc", "Empty Document").await;

        let result = ingest_chunks(&db, "empty-doc", vec![]).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_ingest_chunks_with_all_fields() {
        let (db, _temp) = setup_test_db().await;
        create_test_library_item(&db, "full-doc", "Full Document").await;

        let chunks = vec![ChunkData {
            content: "Attack of Opportunity rules".to_string(),
            content_type: "rules".to_string(),
            page_number: Some(195),
            page_start: Some(195),
            page_end: Some(196),
            section_path: Some("Chapter 9 > Combat > Reactions".to_string()),
            chapter_title: Some("Combat".to_string()),
            section_title: Some("Reactions".to_string()),
            chunk_type: Some("rule_block".to_string()),
            semantic_keywords: Some(vec![
                "attack".to_string(),
                "opportunity".to_string(),
                "reaction".to_string(),
            ]),
            embedding: Some(vec![0.1; 768]),
            embedding_model: Some("nomic-embed-text-v1.5".to_string()),
            metadata: Some(serde_json::json!({"source": "phb"})),
        }];

        let result = ingest_chunks(&db, "full-doc", chunks).await;
        assert!(result.is_ok(), "Ingest failed: {:?}", result.err());
        assert_eq!(result.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_ingest_chunks_updates_library_status() {
        let (db, _temp) = setup_test_db().await;
        create_test_library_item(&db, "status-doc", "Status Test").await;

        // Verify initial status is pending
        #[derive(Debug, Deserialize)]
        struct StatusResult {
            status: String,
        }

        let initial: Option<StatusResult> = db
            .query("SELECT status FROM type::thing('library_item', 'status-doc')")
            .await
            .unwrap()
            .take(0)
            .unwrap();
        assert_eq!(initial.unwrap().status, "pending");

        // Ingest a chunk
        let chunks = vec![ChunkData {
            content: "Test content".to_string(),
            content_type: "rules".to_string(),
            ..Default::default()
        }];

        ingest_chunks(&db, "status-doc", chunks).await.unwrap();

        // Verify status is now ready
        let final_status: Option<StatusResult> = db
            .query("SELECT status FROM type::thing('library_item', 'status-doc')")
            .await
            .unwrap()
            .take(0)
            .unwrap();
        assert_eq!(final_status.unwrap().status, "ready");
    }

    #[tokio::test]
    async fn test_delete_library_chunks() {
        let (db, _temp) = setup_test_db().await;
        create_test_library_item(&db, "delete-doc", "Delete Test").await;

        // Ingest some chunks
        let chunks = vec![
            ChunkData {
                content: "Chunk 1".to_string(),
                content_type: "rules".to_string(),
                ..Default::default()
            },
            ChunkData {
                content: "Chunk 2".to_string(),
                content_type: "rules".to_string(),
                ..Default::default()
            },
            ChunkData {
                content: "Chunk 3".to_string(),
                content_type: "rules".to_string(),
                ..Default::default()
            },
        ];

        ingest_chunks(&db, "delete-doc", chunks).await.unwrap();
        assert_eq!(get_chunk_count(&db, "delete-doc").await.unwrap(), 3);

        // Delete all chunks
        let deleted = delete_library_chunks(&db, "delete-doc").await.unwrap();
        assert_eq!(deleted, 3);

        // Verify all chunks are gone
        assert_eq!(get_chunk_count(&db, "delete-doc").await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_delete_nonexistent_chunks() {
        let (db, _temp) = setup_test_db().await;
        create_test_library_item(&db, "no-chunks", "No Chunks").await;

        let deleted = delete_library_chunks(&db, "no-chunks").await.unwrap();
        assert_eq!(deleted, 0);
    }

    #[tokio::test]
    async fn test_ingest_chunks_with_embeddings() {
        let (db, _temp) = setup_test_db().await;
        create_test_library_item(&db, "embed-doc", "Embedding Test").await;

        let chunks = vec![
            ChunkData {
                content: "First chunk".to_string(),
                content_type: "rules".to_string(),
                ..Default::default()
            },
            ChunkData {
                content: "Second chunk".to_string(),
                content_type: "rules".to_string(),
                ..Default::default()
            },
        ];

        let embeddings = vec![vec![0.1f32; 768], vec![0.2f32; 768]];

        let result = ingest_chunks_with_embeddings(&db, "embed-doc", chunks, embeddings).await;
        assert!(result.is_ok(), "Ingest with embeddings failed: {:?}", result.err());
        assert_eq!(result.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_ingest_chunks_with_embeddings_mismatch() {
        let (db, _temp) = setup_test_db().await;
        create_test_library_item(&db, "mismatch-doc", "Mismatch Test").await;

        let chunks = vec![
            ChunkData {
                content: "First chunk".to_string(),
                content_type: "rules".to_string(),
                ..Default::default()
            },
            ChunkData {
                content: "Second chunk".to_string(),
                content_type: "rules".to_string(),
                ..Default::default()
            },
        ];

        // Only one embedding for two chunks
        let embeddings = vec![vec![0.1f32; 768]];

        let result = ingest_chunks_with_embeddings(&db, "mismatch-doc", chunks, embeddings).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(StorageError::Config(_))));
    }

    #[tokio::test]
    async fn test_get_chunk_count() {
        let (db, _temp) = setup_test_db().await;
        create_test_library_item(&db, "count-doc", "Count Test").await;

        // Initially no chunks
        assert_eq!(get_chunk_count(&db, "count-doc").await.unwrap(), 0);

        // Add chunks
        let chunks: Vec<ChunkData> = (0..5)
            .map(|i| ChunkData {
                content: format!("Chunk {}", i),
                content_type: "rules".to_string(),
                ..Default::default()
            })
            .collect();

        ingest_chunks(&db, "count-doc", chunks).await.unwrap();
        assert_eq!(get_chunk_count(&db, "count-doc").await.unwrap(), 5);
    }

    #[tokio::test]
    async fn test_update_chunk_embeddings() {
        let (db, _temp) = setup_test_db().await;
        create_test_library_item(&db, "update-embed", "Update Embedding Test").await;

        // Ingest chunks without embeddings
        let chunks = vec![ChunkData {
            content: "Test chunk".to_string(),
            content_type: "rules".to_string(),
            ..Default::default()
        }];

        ingest_chunks(&db, "update-embed", chunks).await.unwrap();

        // Update with embeddings
        let chunk_ids = vec!["update-embed-0".to_string()];
        let embeddings = vec![vec![0.5f32; 768]];

        let updated = update_chunk_embeddings(&db, chunk_ids, embeddings, "test-model")
            .await
            .unwrap();
        assert_eq!(updated, 1);
    }

    #[tokio::test]
    async fn test_chunk_data_serialization() {
        let chunk = ChunkData {
            content: "Test content".to_string(),
            content_type: "rules".to_string(),
            page_number: Some(42),
            semantic_keywords: Some(vec!["test".to_string()]),
            ..Default::default()
        };

        let json = serde_json::to_string(&chunk).unwrap();
        let deserialized: ChunkData = serde_json::from_str(&json).unwrap();

        assert_eq!(chunk.content, deserialized.content);
        assert_eq!(chunk.page_number, deserialized.page_number);
    }

    #[tokio::test]
    async fn test_chunk_data_default() {
        let chunk = ChunkData::default();

        assert!(chunk.content.is_empty());
        assert!(chunk.content_type.is_empty());
        assert!(chunk.page_number.is_none());
        assert!(chunk.embedding.is_none());
    }
}
