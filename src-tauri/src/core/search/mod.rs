//! Search Module
//!
//! Provides hybrid search capabilities combining keyword and semantic search
//! with TTRPG-specific query enhancement.
//!
//! # Architecture
//!
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
//! use crate::core::search::{HybridSearchEngine, HybridConfig, HybridSearchOptions};
//!
//! let engine = HybridSearchEngine::new(
//!     search_client,
//!     Some(embedding_provider),
//!     HybridConfig::balanced(),
//! );
//!
//! let results = engine.search("fireball damage", HybridSearchOptions::default()).await?;
//! ```

pub mod embeddings;
pub mod fusion;
pub mod hybrid;
pub mod providers;
pub mod synonyms;
pub mod query;

// Re-export commonly used types
pub use embeddings::{EmbeddingProvider, EmbeddingCache, EmbeddingConfig, EmbeddingError};
pub use fusion::{RRFEngine, RRFConfig, FusionStrategy, FusedSearchResult};
pub use hybrid::{
    HybridSearchEngine, HybridConfig, HybridSearchResult, HybridSearchResponse,
    HybridSearchOptions, HybridSearchError,
};
pub use providers::{create_provider, OllamaEmbeddings, OpenAIEmbeddings};
pub use synonyms::{TTRPGSynonyms, QueryExpansionResult, ClarificationPrompt, ExpansionInfo, DiceNotation};
pub use query::{
    QueryEnhancer, EnhancedQuery, CorrectionDetails, ExpansionDetails,
    SearchHint, HintType, WordCorrection, TermExpansion,
    enhance_query, get_query_suggestions, get_query_hints,
};
