//! LLM Provider Trait
//!
//! Defines the trait that all LLM providers must implement.

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::core::llm::cost::ProviderPricing;
use super::error::{LLMError, Result};
use super::types::{ChatChunk, ChatRequest, ChatResponse};

/// Trait that all LLM providers must implement
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Get the provider's unique identifier
    fn id(&self) -> &str;

    /// Get the provider's display name
    fn name(&self) -> &str;

    /// Get the model being used
    fn model(&self) -> &str;

    /// Check if the provider is healthy/available
    async fn health_check(&self) -> bool;

    /// Get pricing information for this provider/model
    fn pricing(&self) -> Option<ProviderPricing>;

    /// Send a chat completion request
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse>;

    /// Send a streaming chat request
    /// Returns a receiver that yields ChatChunk events
    async fn stream_chat(
        &self,
        request: ChatRequest,
    ) -> Result<mpsc::Receiver<Result<ChatChunk>>>;

    /// Generate embeddings for the given text
    async fn embeddings(&self, _text: String) -> Result<Vec<f32>> {
        Err(LLMError::EmbeddingNotSupported(self.id().to_string()))
    }

    /// Check if streaming is supported
    fn supports_streaming(&self) -> bool {
        true
    }

    /// Check if embeddings are supported
    fn supports_embeddings(&self) -> bool {
        false
    }
}
