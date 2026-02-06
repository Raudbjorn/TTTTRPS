//! Search Module
//!
//! Provides a unified search interface combining:
//! - Meilisearch-based document indexing and search
//! - Hybrid search capabilities (keyword + semantic)
//! - TTRPG-specific query enhancement and synonyms
//!
//! # Architecture
//!
//! ## Core Search Client (from search_client.rs)
//! - `client`: Core `SearchClient` struct for Meilisearch operations
//! - `config`: Configuration types, constants, and embedder settings
//! - `error`: Error types and Result alias
//! - `library`: Library document repository for metadata persistence
//! - `models`: Data structures for documents and search results
//! - `ttrpg`: TTRPG-specific document types and operations
//!
//! ## Hybrid Search Engine
//! - `embeddings`: Embedding provider trait and cache
//! - `providers`: Concrete embedding providers (Ollama, OpenAI)
//! - `fusion`: Reciprocal Rank Fusion (RRF) algorithm for result merging
//! - `hybrid`: Hybrid search engine combining keyword and semantic search
//! - `synonyms`: TTRPG synonym dictionary for query expansion
//! - `query`: Unified query enhancement with correction, expansion, and suggestions
//!
//! # Usage Example
//!
//! ```rust,ignore
//! use crate::core::search::{SearchClient, HybridSearchEngine, HybridConfig};
//!
//! // Basic Meilisearch search
//! let client = SearchClient::new("http://localhost:7700", Some("key")).unwrap();
//! let results = client.search("fireball", 10, None).await?;
//!
//! // Hybrid search with embeddings
//! let engine = HybridSearchEngine::new(
//!     search_client,
//!     Some(embedding_provider),
//!     HybridConfig::balanced(),
//! );
//! let results = engine.search("fireball damage", HybridSearchOptions::default()).await?;
//! ```

// ============================================================================
// Core Search Client Modules (from search_client.rs refactoring)
// ============================================================================

mod client;
mod config;
pub mod embedded;
mod error;
mod library;
mod models;
mod ttrpg;

// ============================================================================
// Hybrid Search Engine Modules (existing)
// ============================================================================

pub mod embeddings;
pub mod fusion;
pub mod hybrid;
pub mod providers;
pub mod query;
pub mod synonyms;

// ============================================================================
// Re-exports: Core Search Client
// ============================================================================

// Core client
pub use client::SearchClient;

// Embedded search (meilisearch-lib wrapper)
pub use embedded::EmbeddedSearch;

// Error handling
pub use error::{Result, SearchError};

// Configuration
pub use config::{
    all_indexes, build_embedder_json, copilot_embedding_dimensions, ollama_embedding_dimensions,
    select_index_for_source_type, EmbedderConfig, DOCUMENT_TEMPLATE_MAX_BYTES, INDEX_CHAT,
    INDEX_DOCUMENTS, INDEX_FICTION, INDEX_LIBRARY_METADATA, INDEX_RULES, TASK_TIMEOUT_LONG_SECS,
    TASK_TIMEOUT_SHORT_SECS, TTRPG_DOCUMENT_TEMPLATE,
};

// Models
pub use models::{
    CoreDocumentFields, FederatedResults, LibraryDocumentMetadata, SearchDocument, SearchResult,
    TTRPGEmbeddingMetadata, TTRPGEnhancedMetadata, TTRPGSemanticMetadata,
};

// TTRPG types
pub use ttrpg::{
    TTRPGFilterableFields, TTRPGSearchDocument, TTRPGSearchResult, INDEX_TTRPG,
    TTRPG_FILTERABLE_ATTRIBUTES, TTRPG_SEARCHABLE_ATTRIBUTES, TTRPG_SORTABLE_ATTRIBUTES,
};

// Library repository trait
pub use library::{LibraryRepository, LibraryRepositoryImpl};

// ============================================================================
// Re-exports: Hybrid Search Engine (existing)
// ============================================================================

pub use embeddings::{EmbeddingCache, EmbeddingConfig, EmbeddingError, EmbeddingProvider};
pub use fusion::{FusedSearchResult, FusionStrategy, RRFConfig, RRFEngine};
pub use hybrid::{
    HybridConfig, HybridSearchEngine, HybridSearchError, HybridSearchOptions,
    HybridSearchResponse, HybridSearchResult,
};
pub use providers::{create_provider, OllamaEmbeddings, OpenAIEmbeddings};
pub use query::{
    enhance_query, get_query_hints, get_query_suggestions, CorrectionDetails, EnhancedQuery,
    ExpansionDetails, HintType, QueryEnhancer, SearchHint, TermExpansion, WordCorrection,
};
pub use synonyms::{
    ClarificationPrompt, DiceNotation, ExpansionInfo, QueryExpansionResult, TTRPGSynonyms,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_selection() {
        assert_eq!(
            SearchClient::select_index_for_source_type("rules"),
            INDEX_RULES
        );
        assert_eq!(
            SearchClient::select_index_for_source_type("fiction"),
            INDEX_FICTION
        );
        assert_eq!(
            SearchClient::select_index_for_source_type("chat"),
            INDEX_CHAT
        );
        assert_eq!(
            SearchClient::select_index_for_source_type("pdf"),
            INDEX_DOCUMENTS
        );
    }

    #[test]
    fn test_ttrpg_index_constant() {
        assert_eq!(INDEX_TTRPG, "ttrpg");
    }
}
