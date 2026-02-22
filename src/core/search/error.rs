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

impl From<meilisearch_sdk::errors::Error> for SearchError {
    fn from(e: meilisearch_sdk::errors::Error) -> Self {
        SearchError::MeilisearchError(e.to_string())
    }
}

impl From<meilisearch_lib::Error> for SearchError {
    fn from(e: meilisearch_lib::Error) -> Self {
        match &e {
            meilisearch_lib::Error::IndexNotFound(uid) => SearchError::IndexNotFound(uid.clone()),
            meilisearch_lib::Error::DocumentNotFound(id) => {
                SearchError::DocumentNotFound(id.clone())
            }
            meilisearch_lib::Error::ChatNotConfigured => SearchError::RagNotConfigured,
            meilisearch_lib::Error::ChatProvider(msg) => SearchError::LlmProvider(msg.clone()),
            meilisearch_lib::Error::Config(msg) => SearchError::ConfigError(msg.clone()),
            meilisearch_lib::Error::MissingDbPath => {
                SearchError::ConfigError("database path is required".to_string())
            }
            _ => SearchError::MeilisearchError(e.to_string()),
        }
    }
}

/// Result type alias for search operations
pub type Result<T> = std::result::Result<T, SearchError>;
