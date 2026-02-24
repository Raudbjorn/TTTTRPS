//! Search Error Types
//!
//! Error handling for the search client module.

use thiserror::Error;

/// Search operation errors
#[derive(Error, Debug)]
pub enum SearchError {
    #[error("Meilisearch error: {0}")]
    MeilisearchError(String),

    #[error("Index not found: {0}")]
    IndexNotFound(String),

    #[error("Document not found: {0}")]
    DocumentNotFound(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Initialization failed: {0}")]
    InitError(String),

    #[error("RAG not configured - call configure_rag first")]
    RagNotConfigured,

    #[error("LLM provider error: {0}")]
    LlmProvider(String),
}

impl From<crate::core::wilysearch::error::Error> for SearchError {
    fn from(e: crate::core::wilysearch::error::Error) -> Self {
        SearchError::MeilisearchError(e.to_string())
    }
}

/// Result type alias for search operations
pub type Result<T> = std::result::Result<T, SearchError>;
