//! OpenRouter Provider Implementation
//!
//! OpenRouter provides access to 400+ models from various providers through a unified API.

use super::openai::OpenAICompatibleProvider;
use crate::core::llm::cost::ProviderPricing;
use crate::core::llm::router::{
    ChatChunk, ChatRequest, ChatResponse, LLMProvider, Result,
};
use async_trait::async_trait;
use tokio::sync::mpsc;

const OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";

/// OpenRouter provider - access to 400+ models
pub struct OpenRouterProvider {
    inner: OpenAICompatibleProvider,
}

impl OpenRouterProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            inner: OpenAICompatibleProvider::new(
                "openrouter".to_string(),
                "OpenRouter".to_string(),
                api_key,
                model,
                4096,
                OPENROUTER_BASE_URL.to_string(),
            ),
        }
    }

    /// Create with a specific model
    pub fn with_model(api_key: String, model: &str) -> Self {
        Self::new(api_key, model.to_string())
    }

    /// Use Claude 3.5 Sonnet via OpenRouter
    pub fn claude_sonnet(api_key: String) -> Self {
        Self::new(api_key, "anthropic/claude-3.5-sonnet".to_string())
    }

    /// Use GPT-4o via OpenRouter
    pub fn gpt4o(api_key: String) -> Self {
        Self::new(api_key, "openai/gpt-4o".to_string())
    }

    /// Use Llama 3.1 70B via OpenRouter
    pub fn llama_70b(api_key: String) -> Self {
        Self::new(api_key, "meta-llama/llama-3.1-70b-instruct".to_string())
    }
}

#[async_trait]
impl LLMProvider for OpenRouterProvider {
    fn id(&self) -> &str {
        "openrouter"
    }

    fn name(&self) -> &str {
        "OpenRouter"
    }

    fn model(&self) -> &str {
        self.inner.model()
    }

    async fn health_check(&self) -> bool {
        self.inner.health_check().await
    }

    fn pricing(&self) -> Option<ProviderPricing> {
        ProviderPricing::for_model("openrouter", self.inner.model())
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let mut response = self.inner.chat(request).await?;
        response.provider = "openrouter".to_string();
        Ok(response)
    }

    async fn stream_chat(
        &self,
        request: ChatRequest,
    ) -> Result<mpsc::Receiver<Result<ChatChunk>>> {
        self.inner.stream_chat(request).await
    }
}
