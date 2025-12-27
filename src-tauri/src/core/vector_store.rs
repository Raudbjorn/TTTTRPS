//! Vector Store Module
//!
//! LanceDB-based vector storage for embeddings with semantic search capabilities.

use lancedb::connect;
use lancedb::connection::Connection;
use lancedb::query::{QueryBase, ExecutableQuery};
use arrow_array::{RecordBatch, RecordBatchIterator, StringArray, Float32Array, FixedSizeListArray, ArrayRef, Array};
use arrow_schema::{Schema, Field, DataType};
use std::sync::Arc;
use thiserror::Error;
use serde::{Deserialize, Serialize};
use futures::TryStreamExt;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum VectorStoreError {
    #[error("LanceDB error: {0}")]
    LanceError(#[from] lancedb::Error),

    #[error("Arrow error: {0}")]
    ArrowError(#[from] arrow_schema::ArrowError),

    #[error("Table not found: {0}")]
    TableNotFound(String),

    #[error("Invalid embedding dimensions: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

pub type Result<T> = std::result::Result<T, VectorStoreError>;

// ============================================================================
// Document Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub content: String,
    pub source: String,
    pub source_type: String,  // "rulebook", "flavor", "campaign"
    pub chunk_index: i32,
    pub page_number: Option<i32>,
    pub metadata: Option<String>, // JSON string for flexible metadata
}

#[derive(Debug, Clone)]
pub struct DocumentWithEmbedding {
    pub document: Document,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub document: Document,
    pub score: f32,
    pub distance: f32,
}

// ============================================================================
// Vector Store Configuration
// ============================================================================

#[derive(Debug, Clone)]
pub struct VectorStoreConfig {
    pub uri: String,
    pub embedding_dimensions: usize,
    pub default_table: String,
}

impl Default for VectorStoreConfig {
    fn default() -> Self {
        Self {
            uri: "./data/vectorstore".to_string(),
            embedding_dimensions: 384, // nomic-embed-text default
            default_table: "documents".to_string(),
        }
    }
}

// ============================================================================
// Vector Store
// ============================================================================

pub struct VectorStore {
    conn: Connection,
    config: VectorStoreConfig,
}

impl VectorStore {
    /// Create a new vector store connection
    pub async fn new(config: VectorStoreConfig) -> Result<Self> {
        let conn = connect(&config.uri).execute().await?;
        Ok(Self { conn, config })
    }

    /// Create with default configuration
    pub async fn with_defaults() -> Result<Self> {
        Self::new(VectorStoreConfig::default()).await
    }

    /// Get the schema for the documents table
    fn get_schema(&self) -> Schema {
        Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new("source", DataType::Utf8, false),
            Field::new("source_type", DataType::Utf8, false),
            Field::new("chunk_index", DataType::Int32, false),
            Field::new("page_number", DataType::Int32, true),
            Field::new("metadata", DataType::Utf8, true),
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    self.config.embedding_dimensions as i32,
                ),
                false,
            ),
        ])
    }

    /// Initialize the documents table (create if not exists)
    pub async fn initialize(&self) -> Result<()> {
        let tables = self.conn.table_names().execute().await?;

        if !tables.contains(&self.config.default_table) {
            // Create empty table with schema
            let schema = Arc::new(self.get_schema());

            // Create empty arrays for initial table
            let id_array = StringArray::from(Vec::<String>::new());
            let content_array = StringArray::from(Vec::<String>::new());
            let source_array = StringArray::from(Vec::<String>::new());
            let source_type_array = StringArray::from(Vec::<String>::new());
            let chunk_index_array = arrow_array::Int32Array::from(Vec::<i32>::new());
            let page_number_array = arrow_array::Int32Array::from(Vec::<Option<i32>>::new());
            let metadata_array = StringArray::from(Vec::<Option<String>>::new());

            // Empty vector array
            let empty_values = Float32Array::from(Vec::<f32>::new());
            let vector_array = FixedSizeListArray::new(
                Arc::new(Field::new("item", DataType::Float32, true)),
                self.config.embedding_dimensions as i32,
                Arc::new(empty_values),
                None,
            );

            let batch = RecordBatch::try_new(
                schema.clone(),
                vec![
                    Arc::new(id_array) as ArrayRef,
                    Arc::new(content_array) as ArrayRef,
                    Arc::new(source_array) as ArrayRef,
                    Arc::new(source_type_array) as ArrayRef,
                    Arc::new(chunk_index_array) as ArrayRef,
                    Arc::new(page_number_array) as ArrayRef,
                    Arc::new(metadata_array) as ArrayRef,
                    Arc::new(vector_array) as ArrayRef,
                ],
            )?;

            let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);
            self.conn
                .create_table(&self.config.default_table, Box::new(batches))
                .execute()
                .await?;

            log::info!("Created table: {}", self.config.default_table);
        }

        Ok(())
    }

    /// Insert documents with embeddings
    pub async fn insert(&self, documents: Vec<DocumentWithEmbedding>) -> Result<usize> {
        if documents.is_empty() {
            return Ok(0);
        }

        // Validate embedding dimensions
        for doc in &documents {
            if doc.embedding.len() != self.config.embedding_dimensions {
                return Err(VectorStoreError::DimensionMismatch {
                    expected: self.config.embedding_dimensions,
                    got: doc.embedding.len(),
                });
            }
        }

        let count = documents.len();
        let schema = Arc::new(self.get_schema());

        // Build arrays
        let ids: Vec<String> = documents.iter().map(|d| d.document.id.clone()).collect();
        let contents: Vec<String> = documents.iter().map(|d| d.document.content.clone()).collect();
        let sources: Vec<String> = documents.iter().map(|d| d.document.source.clone()).collect();
        let source_types: Vec<String> = documents.iter().map(|d| d.document.source_type.clone()).collect();
        let chunk_indices: Vec<i32> = documents.iter().map(|d| d.document.chunk_index).collect();
        let page_numbers: Vec<Option<i32>> = documents.iter().map(|d| d.document.page_number).collect();
        let metadatas: Vec<Option<String>> = documents.iter().map(|d| d.document.metadata.clone()).collect();

        // Flatten embeddings for FixedSizeListArray
        let flat_embeddings: Vec<f32> = documents.iter()
            .flat_map(|d| d.embedding.clone())
            .collect();

        let id_array = StringArray::from(ids);
        let content_array = StringArray::from(contents);
        let source_array = StringArray::from(sources);
        let source_type_array = StringArray::from(source_types);
        let chunk_index_array = arrow_array::Int32Array::from(chunk_indices);
        let page_number_array = arrow_array::Int32Array::from(page_numbers);
        let metadata_array = StringArray::from(metadatas);

        let values_array = Float32Array::from(flat_embeddings);
        let vector_array = FixedSizeListArray::new(
            Arc::new(Field::new("item", DataType::Float32, true)),
            self.config.embedding_dimensions as i32,
            Arc::new(values_array),
            None,
        );

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(id_array) as ArrayRef,
                Arc::new(content_array) as ArrayRef,
                Arc::new(source_array) as ArrayRef,
                Arc::new(source_type_array) as ArrayRef,
                Arc::new(chunk_index_array) as ArrayRef,
                Arc::new(page_number_array) as ArrayRef,
                Arc::new(metadata_array) as ArrayRef,
                Arc::new(vector_array) as ArrayRef,
            ],
        )?;

        let table = self.conn.open_table(&self.config.default_table).execute().await?;
        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);
        table.add(Box::new(batches)).execute().await?;

        log::info!("Inserted {} documents", count);
        Ok(count)
    }

    /// Upsert documents (insert or update by ID)
    pub async fn upsert(&self, documents: Vec<DocumentWithEmbedding>) -> Result<usize> {
        if documents.is_empty() {
            return Ok(0);
        }

        // Delete existing documents with same IDs
        let ids: Vec<&str> = documents.iter().map(|d| d.document.id.as_str()).collect();
        self.delete_by_ids(&ids).await?;

        // Insert new documents
        self.insert(documents).await
    }

    /// Search for similar documents
    pub async fn search(
        &self,
        query_embedding: &[f32],
        limit: usize,
        source_filter: Option<&str>,
    ) -> Result<Vec<SearchResult>> {
        if query_embedding.len() != self.config.embedding_dimensions {
            return Err(VectorStoreError::DimensionMismatch {
                expected: self.config.embedding_dimensions,
                got: query_embedding.len(),
            });
        }

        let table = self.conn.open_table(&self.config.default_table).execute().await?;

        let mut query = table
            .vector_search(query_embedding.to_vec())?
            .limit(limit);

        // Apply source filter if provided
        if let Some(source) = source_filter {
            query = query.only_if(format!("source_type = '{}'", source));
        }

        let results = query.execute().await?;
        let batches: Vec<RecordBatch> = results.try_collect().await?;

        let mut search_results = Vec::new();

        for batch in batches {
            let id_col = batch.column_by_name("id")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let content_col = batch.column_by_name("content")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let source_col = batch.column_by_name("source")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let source_type_col = batch.column_by_name("source_type")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let chunk_index_col = batch.column_by_name("chunk_index")
                .and_then(|c| c.as_any().downcast_ref::<arrow_array::Int32Array>());
            let page_number_col = batch.column_by_name("page_number")
                .and_then(|c| c.as_any().downcast_ref::<arrow_array::Int32Array>());
            let metadata_col = batch.column_by_name("metadata")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let distance_col = batch.column_by_name("_distance")
                .and_then(|c| c.as_any().downcast_ref::<Float32Array>());

            if let (Some(ids), Some(contents), Some(sources), Some(source_types), Some(chunk_indices)) =
                (id_col, content_col, source_col, source_type_col, chunk_index_col) {
                for i in 0..batch.num_rows() {
                    let distance = distance_col.map(|d| d.value(i)).unwrap_or(0.0);
                    let score = 1.0 / (1.0 + distance); // Convert distance to similarity score

                    search_results.push(SearchResult {
                        document: Document {
                            id: ids.value(i).to_string(),
                            content: contents.value(i).to_string(),
                            source: sources.value(i).to_string(),
                            source_type: source_types.value(i).to_string(),
                            chunk_index: chunk_indices.value(i),
                            page_number: page_number_col.map(|p| {
                                if p.is_null(i) { None } else { Some(p.value(i)) }
                            }).flatten(),
                            metadata: metadata_col.and_then(|m| {
                                if m.is_null(i) { None } else { Some(m.value(i).to_string()) }
                            }),
                        },
                        score,
                        distance,
                    });
                }
            }
        }

        Ok(search_results)
    }

    /// Delete documents by IDs
    pub async fn delete_by_ids(&self, ids: &[&str]) -> Result<usize> {
        if ids.is_empty() {
            return Ok(0);
        }

        let table = self.conn.open_table(&self.config.default_table).execute().await?;

        let id_list = ids.iter()
            .map(|id| format!("'{}'", id))
            .collect::<Vec<_>>()
            .join(", ");

        table.delete(&format!("id IN ({})", id_list)).await?;

        log::info!("Deleted {} documents", ids.len());
        Ok(ids.len())
    }

    /// Delete documents by source
    pub async fn delete_by_source(&self, source: &str) -> Result<()> {
        let table = self.conn.open_table(&self.config.default_table).execute().await?;
        table.delete(&format!("source = '{}'", source)).await?;
        log::info!("Deleted documents from source: {}", source);
        Ok(())
    }

    /// Get document count
    pub async fn count(&self) -> Result<usize> {
        let table = self.conn.open_table(&self.config.default_table).execute().await?;
        let count = table.count_rows(None).await?;
        Ok(count)
    }

    /// Get document by ID
    pub async fn get_by_id(&self, id: &str) -> Result<Option<Document>> {
        let table = self.conn.open_table(&self.config.default_table).execute().await?;

        let results = table
            .query()
            .only_if(format!("id = '{}'", id))
            .limit(1)
            .execute()
            .await?;

        let batches: Vec<RecordBatch> = results.try_collect().await?;

        if batches.is_empty() || batches[0].num_rows() == 0 {
            return Ok(None);
        }

        let batch = &batches[0];
        let id_col = batch.column_by_name("id")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>());
        let content_col = batch.column_by_name("content")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>());
        let source_col = batch.column_by_name("source")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>());
        let source_type_col = batch.column_by_name("source_type")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>());
        let chunk_index_col = batch.column_by_name("chunk_index")
            .and_then(|c| c.as_any().downcast_ref::<arrow_array::Int32Array>());
        let page_number_col = batch.column_by_name("page_number")
            .and_then(|c| c.as_any().downcast_ref::<arrow_array::Int32Array>());
        let metadata_col = batch.column_by_name("metadata")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>());

        if let (Some(ids), Some(contents), Some(sources), Some(source_types), Some(chunk_indices)) =
            (id_col, content_col, source_col, source_type_col, chunk_index_col) {
            Ok(Some(Document {
                id: ids.value(0).to_string(),
                content: contents.value(0).to_string(),
                source: sources.value(0).to_string(),
                source_type: source_types.value(0).to_string(),
                chunk_index: chunk_indices.value(0),
                page_number: page_number_col.map(|p| {
                    if p.is_null(0) { None } else { Some(p.value(0)) }
                }).flatten(),
                metadata: metadata_col.and_then(|m| {
                    if m.is_null(0) { None } else { Some(m.value(0).to_string()) }
                }),
            }))
        } else {
            Ok(None)
        }
    }

    /// List all sources
    pub async fn list_sources(&self) -> Result<Vec<String>> {
        let table = self.conn.open_table(&self.config.default_table).execute().await?;

        let results = table
            .query()
            .select(lancedb::query::Select::Columns(vec!["source".to_string()]))
            .execute()
            .await?;

        let batches: Vec<RecordBatch> = results.try_collect().await?;

        let mut sources = std::collections::HashSet::new();
        for batch in batches {
            if let Some(source_col) = batch.column_by_name("source")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>()) {
                for i in 0..batch.num_rows() {
                    sources.insert(source_col.value(i).to_string());
                }
            }
        }

        Ok(sources.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_serialization() {
        let doc = Document {
            id: "test-1".to_string(),
            content: "Test content".to_string(),
            source: "test.pdf".to_string(),
            source_type: "rulebook".to_string(),
            chunk_index: 0,
            page_number: Some(1),
            metadata: Some(r#"{"key": "value"}"#.to_string()),
        };

        let json = serde_json::to_string(&doc).unwrap();
        let parsed: Document = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, doc.id);
        assert_eq!(parsed.content, doc.content);
    }
}
