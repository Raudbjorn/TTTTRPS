//! Keyword Search Module
//!
//! BM25-based full-text search using Tantivy for keyword matching.

use std::path::Path;
use std::sync::RwLock;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Schema, TEXT, STRING, STORED, Field, IndexRecordOption, TextFieldIndexing, TextOptions, Value};
use tantivy::{Index, IndexWriter, ReloadPolicy, TantivyDocument};
use thiserror::Error;
use serde::{Deserialize, Serialize};

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum KeywordSearchError {
    #[error("Tantivy error: {0}")]
    TantivyError(#[from] tantivy::TantivyError),

    #[error("Query parse error: {0}")]
    QueryParseError(#[from] tantivy::query::QueryParserError),

    #[error("Index not initialized")]
    NotInitialized,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, KeywordSearchError>;

// ============================================================================
// Search Result Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordSearchResult {
    pub id: String,
    pub content: String,
    pub source: String,
    pub source_type: String,
    pub chunk_index: i32,
    pub score: f32,
}

// ============================================================================
// Keyword Search Index
// ============================================================================

pub struct KeywordIndex {
    index: Index,
    writer: RwLock<Option<IndexWriter>>,
    schema: Schema,
    // Field references
    id_field: Field,
    content_field: Field,
    source_field: Field,
    source_type_field: Field,
    chunk_index_field: Field,
}

impl KeywordIndex {
    /// Create a new keyword index at the specified path
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        std::fs::create_dir_all(path)?;

        // Build schema
        let mut schema_builder = Schema::builder();

        // ID field - stored for retrieval
        let id_field = schema_builder.add_text_field("id", STRING | STORED);

        // Content field - indexed with BM25 and stored
        let text_indexing = TextFieldIndexing::default()
            .set_tokenizer("default")
            .set_index_option(IndexRecordOption::WithFreqsAndPositions);
        let text_options = TextOptions::default()
            .set_indexing_options(text_indexing)
            .set_stored();
        let content_field = schema_builder.add_text_field("content", text_options);

        // Source fields - stored for filtering/retrieval
        let source_field = schema_builder.add_text_field("source", STRING | STORED);
        let source_type_field = schema_builder.add_text_field("source_type", STRING | STORED);

        // Chunk index - stored as text (Tantivy doesn't have native i32 stored type in simple API)
        let chunk_index_field = schema_builder.add_text_field("chunk_index", STRING | STORED);

        let schema = schema_builder.build();

        // Create or open index
        let index = if path.join("meta.json").exists() {
            Index::open_in_dir(path)?
        } else {
            Index::create_in_dir(path, schema.clone())?
        };

        // Create writer with 50MB buffer
        let writer = index.writer(50_000_000)?;

        Ok(Self {
            index,
            writer: RwLock::new(Some(writer)),
            schema,
            id_field,
            content_field,
            source_field,
            source_type_field,
            chunk_index_field,
        })
    }

    /// Create an in-memory index (for testing)
    pub fn in_memory() -> Result<Self> {
        let mut schema_builder = Schema::builder();

        let id_field = schema_builder.add_text_field("id", STRING | STORED);

        let text_indexing = TextFieldIndexing::default()
            .set_tokenizer("default")
            .set_index_option(IndexRecordOption::WithFreqsAndPositions);
        let text_options = TextOptions::default()
            .set_indexing_options(text_indexing)
            .set_stored();
        let content_field = schema_builder.add_text_field("content", text_options);

        let source_field = schema_builder.add_text_field("source", STRING | STORED);
        let source_type_field = schema_builder.add_text_field("source_type", STRING | STORED);
        let chunk_index_field = schema_builder.add_text_field("chunk_index", STRING | STORED);

        let schema = schema_builder.build();
        let index = Index::create_in_ram(schema.clone());
        let writer = index.writer(50_000_000)?;

        Ok(Self {
            index,
            writer: RwLock::new(Some(writer)),
            schema,
            id_field,
            content_field,
            source_field,
            source_type_field,
            chunk_index_field,
        })
    }

    /// Index a document
    pub fn index_document(
        &self,
        id: &str,
        content: &str,
        source: &str,
        source_type: &str,
        chunk_index: i32,
    ) -> Result<()> {
        let writer_guard = self.writer.read().unwrap();
        let writer = writer_guard.as_ref().ok_or(KeywordSearchError::NotInitialized)?;

        let mut doc = TantivyDocument::default();
        doc.add_text(self.id_field, id);
        doc.add_text(self.content_field, content);
        doc.add_text(self.source_field, source);
        doc.add_text(self.source_type_field, source_type);
        doc.add_text(self.chunk_index_field, &chunk_index.to_string());

        // Use add_document which takes ownership
        drop(writer_guard);
        let mut writer_guard = self.writer.write().unwrap();
        if let Some(writer) = writer_guard.as_mut() {
            writer.add_document(doc)?;
        }

        Ok(())
    }

    /// Index multiple documents in batch
    pub fn index_documents(&self, documents: Vec<(String, String, String, String, i32)>) -> Result<usize> {
        let count = documents.len();

        let mut writer_guard = self.writer.write().unwrap();
        let writer = writer_guard.as_mut().ok_or(KeywordSearchError::NotInitialized)?;

        for (id, content, source, source_type, chunk_index) in documents {
            let mut doc = TantivyDocument::default();
            doc.add_text(self.id_field, &id);
            doc.add_text(self.content_field, &content);
            doc.add_text(self.source_field, &source);
            doc.add_text(self.source_type_field, &source_type);
            doc.add_text(self.chunk_index_field, &chunk_index.to_string());
            writer.add_document(doc)?;
        }

        Ok(count)
    }

    /// Commit pending changes
    pub fn commit(&self) -> Result<()> {
        let mut writer_guard = self.writer.write().unwrap();
        if let Some(writer) = writer_guard.as_mut() {
            writer.commit()?;
        }
        Ok(())
    }

    /// Search for documents matching the query
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<KeywordSearchResult>> {
        let reader = self.index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        let searcher = reader.searcher();

        let query_parser = QueryParser::for_index(&self.index, vec![self.content_field]);
        let parsed_query = query_parser.parse_query(query)?;

        let top_docs = searcher.search(&parsed_query, &TopDocs::with_limit(limit))?;

        let mut results = Vec::new();

        for (score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;

            let id = retrieved_doc
                .get_first(self.id_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let content = retrieved_doc
                .get_first(self.content_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let source = retrieved_doc
                .get_first(self.source_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let source_type = retrieved_doc
                .get_first(self.source_type_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let chunk_index: i32 = retrieved_doc
                .get_first(self.chunk_index_field)
                .and_then(|v| v.as_str())
                .and_then(|s: &str| s.parse().ok())
                .unwrap_or(0);

            results.push(KeywordSearchResult {
                id,
                content,
                source,
                source_type,
                chunk_index,
                score,
            });
        }

        Ok(results)
    }

    /// Search with source type filter
    pub fn search_with_filter(
        &self,
        query: &str,
        source_type_filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<KeywordSearchResult>> {
        let reader = self.index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        let searcher = reader.searcher();

        // Build query with optional filter
        let query_str = match source_type_filter {
            Some(st) => format!("({}) AND source_type:{}", query, st),
            None => query.to_string(),
        };

        let query_parser = QueryParser::for_index(
            &self.index,
            vec![self.content_field, self.source_type_field],
        );
        let parsed_query = query_parser.parse_query(&query_str)?;

        let top_docs = searcher.search(&parsed_query, &TopDocs::with_limit(limit))?;

        let mut results = Vec::new();

        for (score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;

            let id = retrieved_doc
                .get_first(self.id_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let content = retrieved_doc
                .get_first(self.content_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let source = retrieved_doc
                .get_first(self.source_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let source_type = retrieved_doc
                .get_first(self.source_type_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let chunk_index: i32 = retrieved_doc
                .get_first(self.chunk_index_field)
                .and_then(|v| v.as_str())
                .and_then(|s: &str| s.parse().ok())
                .unwrap_or(0);

            results.push(KeywordSearchResult {
                id,
                content,
                source,
                source_type,
                chunk_index,
                score,
            });
        }

        Ok(results)
    }

    /// Delete documents by source
    pub fn delete_by_source(&self, source: &str) -> Result<()> {
        let mut writer_guard = self.writer.write().unwrap();
        let writer = writer_guard.as_mut().ok_or(KeywordSearchError::NotInitialized)?;

        let term = tantivy::Term::from_field_text(self.source_field, source);
        writer.delete_term(term);
        writer.commit()?;

        Ok(())
    }

    /// Delete a document by ID
    pub fn delete_by_id(&self, id: &str) -> Result<()> {
        let mut writer_guard = self.writer.write().unwrap();
        let writer = writer_guard.as_mut().ok_or(KeywordSearchError::NotInitialized)?;

        let term = tantivy::Term::from_field_text(self.id_field, id);
        writer.delete_term(term);

        Ok(())
    }

    /// Get document count (approximate)
    pub fn count(&self) -> Result<u64> {
        let reader = self.index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        let searcher = reader.searcher();
        Ok(searcher.num_docs())
    }

    /// Clear all documents from the index
    pub fn clear(&self) -> Result<()> {
        let mut writer_guard = self.writer.write().unwrap();
        let writer = writer_guard.as_mut().ok_or(KeywordSearchError::NotInitialized)?;

        writer.delete_all_documents()?;
        writer.commit()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_and_search() {
        let index = KeywordIndex::in_memory().unwrap();

        index
            .index_document(
                "doc-1",
                "The goblin attacks with a rusty sword",
                "monster-manual.pdf",
                "rulebook",
                0,
            )
            .unwrap();

        index
            .index_document(
                "doc-2",
                "Dragons breathe fire and fly through the sky",
                "monster-manual.pdf",
                "rulebook",
                1,
            )
            .unwrap();

        index.commit().unwrap();

        let results = index.search("goblin sword", 10).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].id, "doc-1");

        let results = index.search("dragon fire", 10).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].id, "doc-2");
    }

    #[test]
    fn test_batch_indexing() {
        let index = KeywordIndex::in_memory().unwrap();

        let docs = vec![
            (
                "doc-1".to_string(),
                "First document content".to_string(),
                "source1.pdf".to_string(),
                "rulebook".to_string(),
                0,
            ),
            (
                "doc-2".to_string(),
                "Second document content".to_string(),
                "source2.pdf".to_string(),
                "flavor".to_string(),
                1,
            ),
        ];

        let count = index.index_documents(docs).unwrap();
        assert_eq!(count, 2);

        index.commit().unwrap();

        let results = index.search("document", 10).unwrap();
        assert_eq!(results.len(), 2);
    }
}
