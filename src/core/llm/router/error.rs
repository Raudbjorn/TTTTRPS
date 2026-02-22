//! LLM Error Types
//!
//! Defines error types for LLM operations.

/// Errors that can occur during LLM operations
#[derive(Debug, thiserror::Error)]
pub enum LLMError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("API error: {status} - {message}")]
    ApiError { status: u16, message: String },

    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("Rate limited: retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Provider not configured: {0}")]
    NotConfigured(String),

    #[error("Embedding not supported for provider: {0}")]
    EmbeddingNotSupported(String),

    #[error("Streaming not supported for provider: {0}")]
    StreamingNotSupported(String),

    #[error("Budget exceeded: {0}")]
    BudgetExceeded(String),

    #[error("No healthy providers available")]
    NoProvidersAvailable,

    #[error("Request timeout")]
    Timeout,

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Stream canceled")]
    StreamCanceled,

    #[error("Embedding generation failed: {0}")]
    EmbeddingError(String),
}

/// Result type for LLM operations
pub type Result<T> = std::result::Result<T, LLMError>;
