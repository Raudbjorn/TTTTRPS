//! Embedding Providers
//!
//! Concrete implementations of the EmbeddingProvider trait.

pub mod ollama;
pub mod openai;

pub use ollama::OllamaEmbeddings;
pub use openai::OpenAIEmbeddings;

use super::embeddings::{EmbeddingConfig, EmbeddingProvider, EmbeddingError, Result};
use std::sync::Arc;

/// Create an embedding provider from configuration
pub fn create_provider(config: &EmbeddingConfig) -> Result<Arc<dyn EmbeddingProvider>> {
    match config.provider.to_lowercase().as_str() {
        "ollama" => {
            let endpoint = config
                .endpoint
                .clone()
                .unwrap_or_else(|| "http://localhost:11434".to_string());
            Ok(Arc::new(OllamaEmbeddings::new(
                &endpoint,
                &config.model,
                config.dimensions,
            )))
        }
        "openai" => {
            let api_key = config
                .api_key
                .clone()
                .ok_or_else(|| EmbeddingError::NotConfigured("OpenAI API key required".to_string()))?;
            Ok(Arc::new(OpenAIEmbeddings::new(
                &api_key,
                config.model.clone(),
                config.endpoint.clone(),
            )))
        }
        _ => Err(EmbeddingError::NotConfigured(format!(
            "Unknown embedding provider: {}",
            config.provider
        ))),
    }
}
