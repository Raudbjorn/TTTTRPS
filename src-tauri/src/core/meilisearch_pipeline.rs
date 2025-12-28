//! Meilisearch Ingestion Pipeline
//!
//! Handles document parsing, chunking, and indexing into Meilisearch.
//! Supports PDFs, text files, and structured content.

use crate::core::search_client::{SearchClient, SearchDocument, SearchError};
use crate::ingestion::pdf_parser::PDFParser;
use crate::ingestion::epub_parser::EPUBParser;
use crate::ingestion::mobi_parser::MOBIParser;
use crate::ingestion::docx_parser::DOCXParser;
use chrono::Utc;
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

// ============================================================================
// Configuration
// ============================================================================

/// Chunking configuration
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    /// Target chunk size in characters
    pub chunk_size: usize,
    /// Overlap between chunks in characters
    pub chunk_overlap: usize,
    /// Minimum chunk size (don't create tiny chunks)
    pub min_chunk_size: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            chunk_size: 1000,
            chunk_overlap: 200,
            min_chunk_size: 100,
        }
    }
}

/// Pipeline configuration
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Chunking settings
    pub chunk_config: ChunkConfig,
    /// Default source type if not specified
    pub default_source_type: String,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            chunk_config: ChunkConfig::default(),
            default_source_type: "document".to_string(),
        }
    }
}

// ============================================================================
// Pipeline Result
// ============================================================================

/// Result of processing a document
#[derive(Debug, Clone)]
pub struct IngestionResult {
    pub source: String,
    pub total_chunks: usize,
    pub stored_chunks: usize,
    pub failed_chunks: usize,
    pub index_used: String,
}

// ============================================================================
// Meilisearch Pipeline
// ============================================================================

pub struct MeilisearchPipeline {
    config: PipelineConfig,
}

impl MeilisearchPipeline {
    pub fn new(config: PipelineConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(PipelineConfig::default())
    }

    /// Process a file and ingest into Meilisearch
    pub async fn process_file(
        &self,
        search_client: &SearchClient,
        path: &Path,
        source_type: &str,
        campaign_id: Option<&str>,
    ) -> Result<IngestionResult, SearchError> {
        let source_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Extract text based on file type
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let chunks = match extension.to_lowercase().as_str() {
            "pdf" => self.process_pdf(path, &source_name)?,
            "epub" => self.process_epub(path, &source_name)?,
            "mobi" | "azw" | "azw3" => self.process_mobi(path, &source_name)?,
            "docx" => self.process_docx(path, &source_name)?,
            "txt" | "md" | "markdown" => self.process_text_file(path, &source_name)?,
            _ => {
                // Try to read as text
                match std::fs::read_to_string(path) {
                    Ok(content) => self.chunk_text(&content, &source_name, None),
                    Err(e) => {
                        return Err(SearchError::ConfigError(format!(
                            "Cannot read file: {}", e
                        )));
                    }
                }
            }
        };

        // Determine target index
        let index_name = SearchClient::select_index_for_source_type(source_type);

        // Build SearchDocuments
        let now = Utc::now().to_rfc3339();
        let documents: Vec<SearchDocument> = chunks
            .into_iter()
            .enumerate()
            .map(|(i, (content, page))| SearchDocument {
                id: format!("{}-{}", Uuid::new_v4(), i),
                content,
                source: source_name.clone(),
                source_type: source_type.to_string(),
                page_number: page,
                chunk_index: Some(i as u32),
                campaign_id: campaign_id.map(|s| s.to_string()),
                session_id: None,
                created_at: now.clone(),
                metadata: HashMap::new(),
            })
            .collect();

        let total_chunks = documents.len();

        // Ingest into Meilisearch
        search_client.add_documents(index_name, documents).await?;

        log::info!(
            "Ingested {} chunks from '{}' into index '{}'",
            total_chunks, source_name, index_name
        );

        Ok(IngestionResult {
            source: source_name,
            total_chunks,
            stored_chunks: total_chunks,
            failed_chunks: 0,
            index_used: index_name.to_string(),
        })
    }

    /// Process a PDF file
    fn process_pdf(&self, path: &Path, source_name: &str) -> Result<Vec<(String, Option<u32>)>, SearchError> {
        let pages = PDFParser::extract_text_with_pages(path)
            .map_err(|e| SearchError::ConfigError(format!("PDF parsing failed: {}", e)))?;

        let mut all_chunks = Vec::new();

        for (page_num, page_text) in pages {
            let page_chunks = self.chunk_text(&page_text, source_name, Some(page_num as u32));
            all_chunks.extend(page_chunks);
        }

        Ok(all_chunks)
    }

    /// Process an EPUB file
    fn process_epub(&self, path: &Path, source_name: &str) -> Result<Vec<(String, Option<u32>)>, SearchError> {
        let extracted = EPUBParser::extract_structured(path)
            .map_err(|e| SearchError::ConfigError(format!("EPUB parsing failed: {}", e)))?;

        let mut all_chunks = Vec::new();

        for chapter in extracted.chapters {
            // Use chapter index as a pseudo "page number" for reference
            let chapter_num = (chapter.index + 1) as u32;
            let chapter_chunks = self.chunk_text(&chapter.text, source_name, Some(chapter_num));
            all_chunks.extend(chapter_chunks);
        }

        log::info!(
            "Processed EPUB '{}': {} chapters, {} total chunks",
            source_name,
            extracted.chapter_count,
            all_chunks.len()
        );

        Ok(all_chunks)
    }

    /// Process a MOBI/AZW file
    fn process_mobi(&self, path: &Path, source_name: &str) -> Result<Vec<(String, Option<u32>)>, SearchError> {
        let extracted = MOBIParser::extract_structured(path)
            .map_err(|e| SearchError::ConfigError(format!("MOBI parsing failed: {}", e)))?;

        let mut all_chunks = Vec::new();

        for section in extracted.sections {
            // Use section index as pseudo page number
            let section_num = (section.index + 1) as u32;
            let section_chunks = self.chunk_text(&section.text, source_name, Some(section_num));
            all_chunks.extend(section_chunks);
        }

        log::info!(
            "Processed MOBI '{}': {} sections, {} total chunks",
            source_name,
            extracted.section_count,
            all_chunks.len()
        );

        Ok(all_chunks)
    }

    /// Process a DOCX file
    fn process_docx(&self, path: &Path, source_name: &str) -> Result<Vec<(String, Option<u32>)>, SearchError> {
        let extracted = DOCXParser::extract_structured(path)
            .map_err(|e| SearchError::ConfigError(format!("DOCX parsing failed: {}", e)))?;

        // DOCX doesn't have page numbers, chunk the full text
        let chunks = self.chunk_text(&extracted.text, source_name, None);

        log::info!(
            "Processed DOCX '{}': {} paragraphs, {} total chunks",
            source_name,
            extracted.paragraphs.len(),
            chunks.len()
        );

        Ok(chunks)
    }

    /// Process a text file
    fn process_text_file(&self, path: &Path, source_name: &str) -> Result<Vec<(String, Option<u32>)>, SearchError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| SearchError::ConfigError(format!("Failed to read file: {}", e)))?;

        Ok(self.chunk_text(&content, source_name, None))
    }

    /// Chunk text content with overlap
    fn chunk_text(&self, text: &str, _source: &str, page_number: Option<u32>) -> Vec<(String, Option<u32>)> {
        let config = &self.config.chunk_config;
        let mut chunks = Vec::new();
        let text = text.trim();

        if text.is_empty() {
            return chunks;
        }

        // If text is smaller than chunk size, return as single chunk
        if text.len() <= config.chunk_size {
            chunks.push((text.to_string(), page_number));
            return chunks;
        }

        // Split into sentences for smarter chunking
        let sentences: Vec<&str> = text
            .split(|c| c == '.' || c == '!' || c == '?')
            .filter(|s| !s.trim().is_empty())
            .collect();

        let mut current_chunk = String::new();

        for sentence in sentences {
            let sentence = sentence.trim();
            let potential_len = current_chunk.len() + sentence.len() + 2; // +2 for ". "

            if potential_len > config.chunk_size && !current_chunk.is_empty() {
                // Save current chunk
                if current_chunk.len() >= config.min_chunk_size {
                    chunks.push((current_chunk.clone(), page_number));
                }

                // Start new chunk with overlap
                let overlap_start = current_chunk
                    .char_indices()
                    .rev()
                    .take(config.chunk_overlap)
                    .last()
                    .map(|(i, _)| i)
                    .unwrap_or(0);

                current_chunk = current_chunk[overlap_start..].to_string();
            }

            if !current_chunk.is_empty() {
                current_chunk.push_str(". ");
            }
            current_chunk.push_str(sentence);
        }

        // Don't forget the last chunk
        if current_chunk.len() >= config.min_chunk_size {
            chunks.push((current_chunk, page_number));
        }

        chunks
    }

    /// Ingest raw text content directly
    pub async fn ingest_text(
        &self,
        search_client: &SearchClient,
        content: &str,
        source: &str,
        source_type: &str,
        campaign_id: Option<&str>,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<IngestionResult, SearchError> {
        let chunks = self.chunk_text(content, source, None);
        let index_name = SearchClient::select_index_for_source_type(source_type);
        let now = Utc::now().to_rfc3339();

        let documents: Vec<SearchDocument> = chunks
            .into_iter()
            .enumerate()
            .map(|(i, (text, _))| SearchDocument {
                id: format!("{}-{}", Uuid::new_v4(), i),
                content: text,
                source: source.to_string(),
                source_type: source_type.to_string(),
                page_number: None,
                chunk_index: Some(i as u32),
                campaign_id: campaign_id.map(|s| s.to_string()),
                session_id: None,
                created_at: now.clone(),
                metadata: metadata.clone().unwrap_or_default(),
            })
            .collect();

        let total_chunks = documents.len();
        search_client.add_documents(index_name, documents).await?;

        Ok(IngestionResult {
            source: source.to_string(),
            total_chunks,
            stored_chunks: total_chunks,
            failed_chunks: 0,
            index_used: index_name.to_string(),
        })
    }

    /// Ingest chat messages into the chat index
    pub async fn ingest_chat_messages(
        &self,
        search_client: &SearchClient,
        messages: Vec<(String, String)>, // (role, content)
        session_id: &str,
        campaign_id: Option<&str>,
    ) -> Result<IngestionResult, SearchError> {
        let now = Utc::now().to_rfc3339();

        let documents: Vec<SearchDocument> = messages
            .into_iter()
            .enumerate()
            .map(|(i, (role, content))| {
                let mut metadata = HashMap::new();
                metadata.insert("role".to_string(), role);

                SearchDocument {
                    id: format!("{}-{}", session_id, i),
                    content,
                    source: format!("session-{}", session_id),
                    source_type: "chat".to_string(),
                    page_number: None,
                    chunk_index: Some(i as u32),
                    campaign_id: campaign_id.map(|s| s.to_string()),
                    session_id: Some(session_id.to_string()),
                    created_at: now.clone(),
                    metadata,
                }
            })
            .collect();

        let total = documents.len();
        search_client.add_documents("chat", documents).await?;

        Ok(IngestionResult {
            source: format!("session-{}", session_id),
            total_chunks: total,
            stored_chunks: total,
            failed_chunks: 0,
            index_used: "chat".to_string(),
        })
    }
}

impl Default for MeilisearchPipeline {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_text_small() {
        let pipeline = MeilisearchPipeline::with_defaults();
        let chunks = pipeline.chunk_text("Small text.", "test", None);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].0, "Small text.");
    }

    #[test]
    fn test_chunk_text_with_sentences() {
        let pipeline = MeilisearchPipeline::new(PipelineConfig {
            chunk_config: ChunkConfig {
                chunk_size: 50,
                chunk_overlap: 10,
                min_chunk_size: 10,
            },
            ..Default::default()
        });

        let text = "First sentence. Second sentence. Third sentence. Fourth sentence.";
        let chunks = pipeline.chunk_text(text, "test", Some(1));

        assert!(chunks.len() > 1);
        for (_, page) in &chunks {
            assert_eq!(*page, Some(1));
        }
    }
}
