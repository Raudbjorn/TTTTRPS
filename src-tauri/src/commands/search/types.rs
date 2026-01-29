//! Shared Types for Search Commands
//!
//! Contains types used across search command modules.

use serde::{Deserialize, Serialize};

// ============================================================================
// Search Options and Results
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchOptions {
    /// Maximum results to return
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Source type filter
    pub source_type: Option<String>,
    /// Campaign ID filter
    pub campaign_id: Option<String>,
    /// Search specific index only
    pub index: Option<String>,
}

fn default_limit() -> usize {
    10
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            limit: default_limit(),
            source_type: None,
            campaign_id: None,
            index: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResultPayload {
    pub content: String,
    pub source: String,
    pub source_type: String,
    pub page_number: Option<u32>,
    pub score: f32,
    pub index: String,
}

// ============================================================================
// Hybrid Search Options and Results
// ============================================================================

/// Options for hybrid search
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct HybridSearchOptions {
    /// Maximum results to return
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Source type filter
    pub source_type: Option<String>,
    /// Campaign ID filter
    pub campaign_id: Option<String>,
    /// Index to search (None = federated search)
    pub index: Option<String>,
    /// Override semantic weight (0.0 - 1.0)
    pub semantic_weight: Option<f32>,
    /// Override keyword weight (0.0 - 1.0)
    pub keyword_weight: Option<f32>,
    /// Fusion strategy preset: "balanced", "keyword_heavy", "semantic_heavy", etc.
    pub fusion_strategy: Option<String>,
    /// Enable/disable query expansion (default: true)
    pub query_expansion: Option<bool>,
    /// Enable/disable spell correction (default: true)
    pub spell_correction: Option<bool>,
}

/// Hybrid search result for frontend
#[derive(Debug, Serialize, Deserialize)]
pub struct HybridSearchResultPayload {
    pub content: String,
    pub source: String,
    pub source_type: String,
    pub page_number: Option<u32>,
    pub score: f32,
    pub index: String,
    pub keyword_rank: Option<usize>,
    pub semantic_rank: Option<usize>,
    /// Number of search methods that found this result (1 = single, 2 = both)
    pub overlap_count: Option<usize>,
}

/// Hybrid search response for frontend
#[derive(Debug, Serialize, Deserialize)]
pub struct HybridSearchResponsePayload {
    pub results: Vec<HybridSearchResultPayload>,
    pub total_hits: usize,
    pub original_query: String,
    pub expanded_query: Option<String>,
    pub corrected_query: Option<String>,
    pub processing_time_ms: u64,
    pub hints: Vec<String>,
    /// Whether performance target was met (<500ms)
    pub within_target: bool,
}

// ============================================================================
// Ingestion Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct IngestOptions {
    /// Source type: "rules", "fiction", "document", etc.
    #[serde(default = "default_source_type")]
    pub source_type: String,
    /// Campaign ID to associate with
    pub campaign_id: Option<String>,
}

fn default_source_type() -> String {
    "document".to_string()
}

impl Default for IngestOptions {
    fn default() -> Self {
        Self {
            source_type: default_source_type(),
            campaign_id: None,
        }
    }
}

/// Result of two-phase document ingestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoPhaseIngestResult {
    /// Generated slug for this source (used as index name base)
    pub slug: String,
    /// Human-readable source name
    pub source_name: String,
    /// Index containing raw pages
    pub raw_index: String,
    /// Index containing semantic chunks
    pub chunks_index: String,
    /// Number of pages extracted
    pub page_count: usize,
    /// Number of semantic chunks created
    pub chunk_count: usize,
    /// Total characters extracted
    pub total_chars: usize,
    /// Detected game system (if any)
    pub game_system: Option<String>,
    /// Detected content category
    pub content_category: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IngestResult {
    pub page_count: usize,
    pub character_count: usize,
    pub source_name: String,
}

/// Progress event for document ingestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestProgress {
    pub stage: String,
    pub progress: f32,       // 0.0 to 1.0
    pub message: String,
    pub source_name: String,
}

// ============================================================================
// Embeddings Types
// ============================================================================

/// Configure Meilisearch embedder for semantic search
#[derive(Debug, Serialize, Deserialize)]
pub struct EmbedderConfigRequest {
    /// Embedder name (e.g., "default", "openai", "ollama")
    pub name: String,
    /// Provider type: "openAi", "ollama", or "huggingFace"
    pub provider: String,
    /// API key (for OpenAI)
    pub api_key: Option<String>,
    /// Model name (e.g., "text-embedding-3-small", "nomic-embed-text")
    pub model: Option<String>,
    /// Embedding dimensions (e.g., 1536 for OpenAI)
    pub dimensions: Option<u32>,
    /// Base URL (for Ollama)
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetupEmbeddingsResult {
    pub indexes_configured: Vec<String>,
    pub model: String,
    pub dimensions: u32,
    pub host: String,
}

/// Result returned when setting up Copilot embeddings
#[derive(Debug, Serialize, Deserialize)]
pub struct SetupCopilotEmbeddingsResult {
    pub indexes_configured: Vec<String>,
    pub model: String,
    pub dimensions: u32,
    /// URL of the Copilot API endpoint being used
    pub api_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaEmbeddingModel {
    pub name: String,
    pub size: String,
    pub dimensions: u32,
}

/// Local embedding model info (HuggingFace/ONNX - runs locally via Meilisearch)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalEmbeddingModel {
    pub id: String,
    pub name: String,
    pub dimensions: u32,
    pub description: String,
}

// ============================================================================
// Library Types
// ============================================================================

/// Update library document TTRPG metadata fields
#[derive(Debug, Clone, serde::Deserialize)]
pub struct UpdateLibraryDocumentRequest {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game_system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setting: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
}

// ============================================================================
// Meilisearch Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct MeilisearchStatus {
    pub healthy: bool,
    pub host: String,
    pub document_counts: Option<std::collections::HashMap<String, u64>>,
}

// ============================================================================
// Extraction Types
// ============================================================================

/// Extraction settings preset
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtractionPreset {
    pub name: String,
    pub description: String,
    pub settings: crate::ingestion::ExtractionSettings,
}

/// OCR availability status
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OcrAvailability {
    pub tesseract_installed: bool,
    pub pdftoppm_installed: bool,
    pub available_languages: Vec<String>,
    pub external_ocr_ready: bool,
}
