//! Mistral AI Provider Implementation

use super::openai::OpenAICompatibleProvider;
use crate::core::llm::cost::{ProviderPricing, TokenUsage};
use crate::core::llm::router::{
    ChatChunk, ChatRequest, ChatResponse, LLMError, LLMProvider, Result,
};
use async_trait::async_trait;
use tokio::sync::mpsc;

const MISTRAL_BASE_URL: &str = "https://api.mistral.ai/v1";

/// Mistral AI provider
pub struct MistralProvider {
    inner: OpenAICompatibleProvider,
}

impl MistralProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            inner: OpenAICompatibleProvider::new(
                "mistral".to_string(),
                "Mistral AI".to_string(),
                api_key,
                model,
                4096,
                MISTRAL_BASE_URL.to_string(),
            ),
        }
    }

    /// Use Mistral Large
    pub fn large(api_key: String) -> Self {
        Self::new(api_key, "mistral-large-latest".to_string())
    }

    /// Use Mistral Medium
    pub fn medium(api_key: String) -> Self {
        Self::new(api_key, "mistral-medium-latest".to_string())
    }

    /// Use Mistral Small
    pub fn small(api_key: String) -> Self {
        Self::new(api_key, "mistral-small-latest".to_string())
    }

    /// Use Codestral (code-focused model)
    pub fn codestral(api_key: String) -> Self {
        Self::new(api_key, "codestral-latest".to_string())
    }
}

#[async_trait]
impl LLMProvider for MistralProvider {
    fn id(&self) -> &str {
        "mistral"
    }

    fn name(&self) -> &str {
        "Mistral AI"
    }

    fn model(&self) -> &str {
        self.inner.model()
    }

    async fn health_check(&self) -> bool {
        self.inner.health_check().await
    }

    fn pricing(&self) -> Option<ProviderPricing> {
        ProviderPricing::for_model("mistral", self.inner.model())
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let mut response = self.inner.chat(request).await?;
        response.provider = "mistral".to_string();
        Ok(response)
    }

    async fn stream_chat(
        &self,
        request: ChatRequest,
    ) -> Result<mpsc::Receiver<Result<ChatChunk>>> {
        self.inner.stream_chat(request).await
    }
}
