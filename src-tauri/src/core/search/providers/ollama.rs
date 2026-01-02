//! Ollama Embeddings Provider
//!
//! Generates embeddings using a local Ollama instance.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::core::search::embeddings::{EmbeddingError, EmbeddingProvider, Result};

// ============================================================================
// Ollama API Types
// ============================================================================

#[derive(Debug, Serialize)]
struct OllamaEmbeddingRequest {
    model: String,
    prompt: String,
}

#[derive(Debug, Deserialize)]
struct OllamaEmbeddingResponse {
    embedding: Vec<f32>,
}

#[derive(Debug, Serialize)]
struct OllamaBatchEmbeddingRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct OllamaBatchEmbeddingResponse {
    embeddings: Vec<Vec<f32>>,
}

// ============================================================================
// Ollama Provider
// ============================================================================

/// Ollama-based embedding provider for local embeddings
pub struct OllamaEmbeddings {
    client: reqwest::Client,
    base_url: String,
    model: String,
    dimensions: usize,
}

impl OllamaEmbeddings {
    /// Create a new Ollama embeddings provider
    ///
    /// # Arguments
    /// * `base_url` - Ollama API endpoint (e.g., "http://localhost:11434")
    /// * `model` - Model name (e.g., "nomic-embed-text", "mxbai-embed-large")
    /// * `dimensions` - Expected embedding dimensions (None for auto-detect)
    pub fn new(base_url: &str, model: &str, dimensions: Option<usize>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            dimensions: dimensions.unwrap_or(768), // Default for nomic-embed-text
        }
    }

    /// Get dimensions for common Ollama embedding models
    fn model_dimensions(model: &str) -> usize {
        match model {
            "nomic-embed-text" => 768,
            "mxbai-embed-large" => 1024,
            "all-minilm" => 384,
            "snowflake-arctic-embed" => 1024,
            _ => 768, // Default
        }
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaEmbeddings {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let url = format!("{}/api/embeddings", self.base_url);

        let request = OllamaEmbeddingRequest {
            model: self.model.clone(),
            prompt: text.to_string(),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(EmbeddingError::ApiError(format!(
                "Ollama API error {}: {}",
                status, error_text
            )));
        }

        let result: OllamaEmbeddingResponse = response
            .json()
            .await
            .map_err(|e| EmbeddingError::InvalidResponse(e.to_string()))?;

        Ok(result.embedding)
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        // Ollama's batch embedding endpoint (if available) or sequential fallback
        let url = format!("{}/api/embed", self.base_url);

        let request = OllamaBatchEmbeddingRequest {
            model: self.model.clone(),
            input: texts.iter().map(|s| s.to_string()).collect(),
        };

        // Try batch endpoint first
        let response = self.client.post(&url).json(&request).send().await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let result: OllamaBatchEmbeddingResponse = resp
                    .json()
                    .await
                    .map_err(|e| EmbeddingError::InvalidResponse(e.to_string()))?;
                Ok(result.embeddings)
            }
            _ => {
                // Fallback to sequential embedding
                let mut embeddings = Vec::with_capacity(texts.len());
                for text in texts {
                    embeddings.push(self.embed(text).await?);
                }
                Ok(embeddings)
            }
        }
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn name(&self) -> &str {
        "ollama"
    }

    async fn health_check(&self) -> bool {
        let url = format!("{}/api/tags", self.base_url);
        match self.client.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_dimensions() {
        assert_eq!(OllamaEmbeddings::model_dimensions("nomic-embed-text"), 768);
        assert_eq!(OllamaEmbeddings::model_dimensions("mxbai-embed-large"), 1024);
    }

    #[test]
    fn test_provider_creation() {
        let provider = OllamaEmbeddings::new(
            "http://localhost:11434",
            "nomic-embed-text",
            Some(768),
        );
        assert_eq!(provider.name(), "ollama");
        assert_eq!(provider.dimensions(), 768);
    }
}
