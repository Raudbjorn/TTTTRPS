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
}

impl From<meilisearch_sdk::errors::Error> for SearchError {
    fn from(e: meilisearch_sdk::errors::Error) -> Self {
        SearchError::MeilisearchError(e.to_string())
    }
}

/// Result type alias for search operations
pub type Result<T> = std::result::Result<T, SearchError>;
