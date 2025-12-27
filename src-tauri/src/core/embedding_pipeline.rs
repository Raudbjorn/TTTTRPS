//! Embedding Generation Pipeline
//!
//! Orchestrates document ingestion, chunking, and embedding generation
//! using configured LLM providers.

use crate::core::llm::{LLMClient, LLMConfig, LLMError};
use crate::core::vector_store::{VectorStore, Document, DocumentWithEmbedding, VectorStoreError};
use crate::ingestion::chunker::{SemanticChunker, ChunkConfig};
use crate::ingestion::pdf_parser::PDFParser;
use crate::ingestion::epub_parser::EPUBParser;
use std::path::Path;
use thiserror::Error;
use serde::{Deserialize, Serialize};

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum PipelineError {
    #[error("LLM error: {0}")]
    LLMError(#[from] LLMError),

    #[error("Vector store error: {0}")]
    VectorStoreError(#[from] VectorStoreError),

    #[error("PDF parsing error: {0}")]
    PDFError(String),

    #[error("EPUB parsing error: {0}")]
    EPUBError(String),

    #[error("Unsupported file format: {0}")]
    UnsupportedFormat(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, PipelineError>;

// ============================================================================
// Pipeline Configuration
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Chunk configuration
    pub chunk_config: ChunkConfigSerializable,
    /// Batch size for embedding generation
    pub batch_size: usize,
    /// Whether to show progress
    pub show_progress: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkConfigSerializable {
    pub target_size: usize,
    pub min_size: usize,
    pub max_size: usize,
    pub overlap_size: usize,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            chunk_config: ChunkConfigSerializable {
                target_size: 1000,
                min_size: 200,
                max_size: 2000,
                overlap_size: 100,
            },
            batch_size: 10,
            show_progress: true,
        }
    }
}

impl From<ChunkConfigSerializable> for ChunkConfig {
    fn from(c: ChunkConfigSerializable) -> Self {
        ChunkConfig {
            target_size: c.target_size,
            min_size: c.min_size,
            max_size: c.max_size,
            overlap_size: c.overlap_size,
            preserve_sentences: true,
            preserve_paragraphs: true,
        }
    }
}

// ============================================================================
// Pipeline Progress
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineProgress {
    pub stage: String,
    pub current: usize,
    pub total: usize,
    pub message: String,
}

// ============================================================================
// Embedding Pipeline
// ============================================================================

pub struct EmbeddingPipeline {
    llm_client: LLMClient,
    vector_store: VectorStore,
    config: PipelineConfig,
}

impl EmbeddingPipeline {
    /// Create a new embedding pipeline
    pub fn new(
        llm_config: LLMConfig,
        vector_store: VectorStore,
        config: PipelineConfig,
    ) -> Self {
        Self {
            llm_client: LLMClient::new(llm_config),
            vector_store,
            config,
        }
    }

    /// Process a document file
    pub async fn process_file(
        &self,
        path: &Path,
        source_type: &str,
    ) -> Result<ProcessResult> {
        let extension = path.extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        let source_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        match extension.as_str() {
            "pdf" => self.process_pdf(path, &source_name, source_type).await,
            "epub" => self.process_epub(path, &source_name, source_type).await,
            "txt" | "md" => self.process_text_file(path, &source_name, source_type).await,
            _ => Err(PipelineError::UnsupportedFormat(extension)),
        }
    }

    /// Process a PDF file
    async fn process_pdf(
        &self,
        path: &Path,
        source_name: &str,
        source_type: &str,
    ) -> Result<ProcessResult> {
        log::info!("Processing PDF: {}", source_name);

        // Extract text with page info
        let pages = PDFParser::extract_text_with_pages(path)
            .map_err(|e| PipelineError::PDFError(e.to_string()))?;

        // Convert to the format expected by chunker
        let pages_converted: Vec<(u32, String)> = pages;

        self.process_pages(&pages_converted, source_name, source_type).await
    }

    /// Process an EPUB file
    async fn process_epub(
        &self,
        path: &Path,
        source_name: &str,
        source_type: &str,
    ) -> Result<ProcessResult> {
        log::info!("Processing EPUB: {}", source_name);

        let extracted = EPUBParser::extract_structured(path)
            .map_err(|e| PipelineError::EPUBError(e.to_string()))?;

        // Convert chapters to pages format (chapter index as page number)
        let pages: Vec<(u32, String)> = extracted.chapters
            .into_iter()
            .map(|ch| (ch.index as u32 + 1, ch.text))
            .collect();

        self.process_pages(&pages, source_name, source_type).await
    }

    /// Process a plain text file
    async fn process_text_file(
        &self,
        path: &Path,
        source_name: &str,
        source_type: &str,
    ) -> Result<ProcessResult> {
        log::info!("Processing text file: {}", source_name);

        let content = std::fs::read_to_string(path)?;
        let pages = vec![(1u32, content)];

        self.process_pages(&pages, source_name, source_type).await
    }

    /// Process pages into chunks and generate embeddings
    async fn process_pages(
        &self,
        pages: &[(u32, String)],
        source_name: &str,
        source_type: &str,
    ) -> Result<ProcessResult> {
        // Create chunker
        let chunker = SemanticChunker::with_config(self.config.chunk_config.clone().into());

        // Chunk the content
        let chunks = chunker.chunk_with_pages(pages, source_name);
        let total_chunks = chunks.len();

        log::info!("Created {} chunks from {}", total_chunks, source_name);

        // Generate embeddings in batches
        let mut documents_with_embeddings = Vec::with_capacity(total_chunks);
        let mut failed_chunks = 0;

        for (batch_idx, batch) in chunks.chunks(self.config.batch_size).enumerate() {
            log::debug!(
                "Processing batch {}/{}",
                batch_idx + 1,
                (total_chunks + self.config.batch_size - 1) / self.config.batch_size
            );

            for chunk in batch {
                match self.generate_embedding(&chunk.content).await {
                    Ok(embedding) => {
                        let doc = Document {
                            id: chunk.id.clone(),
                            content: chunk.content.clone(),
                            source: source_name.to_string(),
                            source_type: source_type.to_string(),
                            chunk_index: chunk.chunk_index as i32,
                            page_number: chunk.page_number.map(|p| p as i32),
                            metadata: chunk.section.as_ref().map(|s| {
                                serde_json::json!({"section": s}).to_string()
                            }),
                        };

                        documents_with_embeddings.push(DocumentWithEmbedding {
                            document: doc,
                            embedding,
                        });
                    }
                    Err(e) => {
                        log::warn!("Failed to generate embedding for chunk {}: {}", chunk.id, e);
                        failed_chunks += 1;
                    }
                }
            }
        }

        // Store in vector database
        let stored_count = self.vector_store
            .upsert(documents_with_embeddings)
            .await?;

        log::info!(
            "Stored {} documents from {} ({} failed)",
            stored_count, source_name, failed_chunks
        );

        Ok(ProcessResult {
            source: source_name.to_string(),
            total_chunks,
            stored_chunks: stored_count,
            failed_chunks,
        })
    }

    /// Generate embedding for text
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let response = self.llm_client.embed(text).await?;
        Ok(response.embedding)
    }

    /// Process raw text directly
    pub async fn process_text(
        &self,
        text: &str,
        source_name: &str,
        source_type: &str,
    ) -> Result<ProcessResult> {
        let pages = vec![(1u32, text.to_string())];
        self.process_pages(&pages, source_name, source_type).await
    }

    /// Delete all documents from a source
    pub async fn delete_source(&self, source: &str) -> Result<()> {
        self.vector_store.delete_by_source(source).await?;
        log::info!("Deleted all documents from source: {}", source);
        Ok(())
    }

    /// Get list of indexed sources
    pub async fn list_sources(&self) -> Result<Vec<String>> {
        Ok(self.vector_store.list_sources().await?)
    }

    /// Get document count
    pub async fn get_document_count(&self) -> Result<usize> {
        Ok(self.vector_store.count().await?)
    }
}

// ============================================================================
// Process Result
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessResult {
    pub source: String,
    pub total_chunks: usize,
    pub stored_chunks: usize,
    pub failed_chunks: usize,
}

// ============================================================================
// Batch Processing
// ============================================================================

/// Process multiple files
pub async fn process_files(
    pipeline: &EmbeddingPipeline,
    files: &[(&Path, &str)], // (path, source_type)
) -> Vec<Result<ProcessResult>> {
    let mut results = Vec::with_capacity(files.len());

    for (path, source_type) in files {
        let result = pipeline.process_file(path, source_type).await;
        results.push(result);
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_config_default() {
        let config = PipelineConfig::default();
        assert_eq!(config.batch_size, 10);
        assert_eq!(config.chunk_config.target_size, 1000);
    }
}
