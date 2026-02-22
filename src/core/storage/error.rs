//! Error types for the storage module.
//!
//! Provides a unified error type for all storage operations, including database
//! errors, query failures, schema migrations, and RAG pipeline errors.

use thiserror::Error;

/// Unified error type for storage operations.
#[derive(Debug, Error)]
pub enum StorageError {
    /// Database connection or operation error.
    #[error("Database error: {0}")]
    Database(String),

    /// Configuration error (invalid settings, missing values).
    #[error("Configuration error: {0}")]
    Config(String),

    /// Initialization failure (startup, connection).
    #[error("Initialization failed: {0}")]
    Init(String),

    /// Query execution error (invalid syntax, timeout).
    #[error("Query error: {0}")]
    Query(String),

    /// Schema migration failure.
    #[error("Schema migration failed: {0}")]
    Migration(String),

    /// Record not found in database.
    #[error("Record not found: {0}")]
    NotFound(String),

    /// Embedding generation or vector operation error.
    #[error("Embedding error: {0}")]
    Embedding(String),

    /// LLM-related error (for RAG operations).
    #[error("LLM error: {0}")]
    LlmError(String),

    /// JSON serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Document extraction or parsing error.
    #[error("Document extraction error: {0}")]
    Extraction(String),

    /// Chunking or text processing error.
    #[error("Chunking error: {0}")]
    Chunking(String),

    /// Index operation error (create, update, delete).
    #[error("Index error: {0}")]
    Index(String),

    /// Transaction error (commit, rollback).
    #[error("Transaction error: {0}")]
    Transaction(String),

    /// Permission or authorization error.
    #[error("Permission denied: {0}")]
    Permission(String),

    /// Resource limit exceeded (storage, memory, connections).
    #[error("Resource limit exceeded: {0}")]
    ResourceLimit(String),

    /// IO error for file operations.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl StorageError {
    /// Create a database error with the given message.
    pub fn database(msg: impl Into<String>) -> Self {
        Self::Database(msg.into())
    }

    /// Create a configuration error with the given message.
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create an initialization error with the given message.
    pub fn init(msg: impl Into<String>) -> Self {
        Self::Init(msg.into())
    }

    /// Create a query error with the given message.
    pub fn query(msg: impl Into<String>) -> Self {
        Self::Query(msg.into())
    }

    /// Create a migration error with the given message.
    pub fn migration(msg: impl Into<String>) -> Self {
        Self::Migration(msg.into())
    }

    /// Create a not found error with the given message.
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    /// Create an embedding error with the given message.
    pub fn embedding(msg: impl Into<String>) -> Self {
        Self::Embedding(msg.into())
    }

    /// Create an LLM error with the given message.
    pub fn llm(msg: impl Into<String>) -> Self {
        Self::LlmError(msg.into())
    }
}

/// Result type alias for storage operations.
pub type StorageResult<T> = Result<T, StorageError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = StorageError::database("connection failed");
        assert_eq!(err.to_string(), "Database error: connection failed");

        let err = StorageError::not_found("document:abc123");
        assert_eq!(err.to_string(), "Record not found: document:abc123");
    }

    #[test]
    fn test_error_constructors() {
        let err = StorageError::config("invalid port");
        assert!(matches!(err, StorageError::Config(_)));

        let err = StorageError::query("syntax error at position 42");
        assert!(matches!(err, StorageError::Query(_)));
    }

    #[test]
    fn test_serde_json_error_conversion() {
        let json_err = serde_json::from_str::<String>("invalid").unwrap_err();
        let storage_err: StorageError = json_err.into();
        assert!(matches!(storage_err, StorageError::Serialization(_)));
    }
}
