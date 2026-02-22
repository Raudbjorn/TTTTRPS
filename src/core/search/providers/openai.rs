//! OpenAI Embeddings Provider
//!
//! Generates embeddings using OpenAI's text-embedding API.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::core::search::embeddings::{EmbeddingError, EmbeddingProvider, Result};

// ============================================================================
// OpenAI API Types
// ============================================================================

#[derive(Debug, Serialize)]
struct OpenAIEmbeddingRequest {
    model: String,
    input: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dimensions: Option<usize>,
}


#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenAIEmbeddingResponse {
    data: Vec<EmbeddingData>,
    _model: String,
    _usage: Usage,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct EmbeddingData {
    embedding: Vec<f32>,
    _index: usize,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Usage {
    _prompt_tokens: u32,
    _total_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct OpenAIError {
    error: OpenAIErrorDetail,
}

#[derive(Debug, Deserialize)]
struct OpenAIErrorDetail {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
}

// ============================================================================
// OpenAI Provider
// ============================================================================

/// OpenAI-based embedding provider
pub struct OpenAIEmbeddings {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
    dimensions: usize,
}

impl OpenAIEmbeddings {
    /// Create a new OpenAI embeddings provider
    ///
    /// # Arguments
    /// * `api_key` - OpenAI API key
    /// * `model` - Model name (e.g., "text-embedding-3-small", "text-embedding-3-large")
    /// * `base_url` - Custom API endpoint (None for OpenAI default)
    pub fn new(api_key: &str, model: String, base_url: Option<String>) -> Self {
        let dimensions = Self::model_dimensions(&model);

        Self {
            client: reqwest::Client::new(),
            api_key: api_key.to_string(),
            model,
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
            dimensions,
        }
    }

    /// Get dimensions for OpenAI embedding models
    #[allow(dead_code)]
    fn model_dimensions(model: &str) -> usize {
        match model {
            "text-embedding-3-small" => 1536,
            "text-embedding-3-large" => 3072,
            "text-embedding-ada-002" => 1536,
            _ => 1536, // Default
        }
    }

    /// Create with custom dimensions (for text-embedding-3-* models)
    pub fn with_dimensions(
        api_key: &str,
        model: String,
        dimensions: usize,
        base_url: Option<String>,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.to_string(),
            model,
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
            dimensions,
        }
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAIEmbeddings {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let embeddings = self.embed_batch(&[text]).await?;
        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| EmbeddingError::InvalidResponse("Empty response".to_string()))
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let url = format!("{}/embeddings", self.base_url);

        let request = OpenAIEmbeddingRequest {
            model: self.model.clone(),
            input: texts.iter().map(|s| s.to_string()).collect(),
            dimensions: if self.model.starts_with("text-embedding-3") {
                Some(self.dimensions)
            } else {
                None
            },
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            // Handle rate limiting
            if status.as_u16() == 429 {
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(60);
                return Err(EmbeddingError::RateLimited(retry_after));
            }

            // Parse error response
            let error_text = response.text().await.unwrap_or_default();
            if let Ok(error) = serde_json::from_str::<OpenAIError>(&error_text) {
                return Err(EmbeddingError::ApiError(format!(
                    "{}: {}",
                    error.error.error_type, error.error.message
                )));
            }
            return Err(EmbeddingError::ApiError(format!(
                "OpenAI API error {}: {}",
                status, error_text
            )));
        }

        let result: OpenAIEmbeddingResponse = response
            .json()
            .await
            .map_err(|e| EmbeddingError::InvalidResponse(e.to_string()))?;

        // Sort by index to ensure correct order
        let mut embeddings: Vec<_> = result.data.into_iter().collect();
        embeddings.sort_by_key(|e| e._index);

        Ok(embeddings.into_iter().map(|e| e.embedding).collect())
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn name(&self) -> &str {
        "openai"
    }

    async fn health_check(&self) -> bool {
        // Simple check - try to list models
        let url = format!("{}/models", self.base_url);
        match self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
        {
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
        assert_eq!(
            OpenAIEmbeddings::model_dimensions("text-embedding-3-small"),
            1536
        );
        assert_eq!(
            OpenAIEmbeddings::model_dimensions("text-embedding-3-large"),
            3072
        );
        assert_eq!(
            OpenAIEmbeddings::model_dimensions("text-embedding-ada-002"),
            1536
        );
    }

    #[test]
    fn test_provider_creation() {
        let provider = OpenAIEmbeddings::new(
            "test-key",
            "text-embedding-3-small".to_string(),
            None,
        );
        assert_eq!(provider.name(), "openai");
        assert_eq!(provider.dimensions(), 1536);
    }

    #[test]
    fn test_custom_dimensions() {
        let provider = OpenAIEmbeddings::with_dimensions(
            "test-key",
            "text-embedding-3-large".to_string(),
            1024,
            None,
        );
        assert_eq!(provider.dimensions(), 1024);
    }
}
