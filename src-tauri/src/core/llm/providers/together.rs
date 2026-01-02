//! Together AI Provider Implementation
//!
//! Together AI provides access to open-source models including the largest Llama models.

use super::openai::OpenAICompatibleProvider;
use crate::core::llm::cost::ProviderPricing;
use crate::core::llm::router::{
    ChatChunk, ChatRequest, ChatResponse, LLMProvider, Result,
};
use async_trait::async_trait;
use tokio::sync::mpsc;

const TOGETHER_BASE_URL: &str = "https://api.together.xyz/v1";

/// Together AI provider
pub struct TogetherProvider {
    inner: OpenAICompatibleProvider,
}

impl TogetherProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            inner: OpenAICompatibleProvider::new(
                "together".to_string(),
                "Together AI".to_string(),
                api_key,
                model,
                4096,
                TOGETHER_BASE_URL.to_string(),
            ),
        }
    }

    /// Use Llama 3.1 405B (largest open-source model)
    pub fn llama_405b(api_key: String) -> Self {
        Self::new(
            api_key,
            "meta-llama/Meta-Llama-3.1-405B-Instruct-Turbo".to_string(),
        )
    }

    /// Use Llama 3.1 70B
    pub fn llama_70b(api_key: String) -> Self {
        Self::new(
            api_key,
            "meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo".to_string(),
        )
    }

    /// Use Llama 3.1 8B
    pub fn llama_8b(api_key: String) -> Self {
        Self::new(
            api_key,
            "meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo".to_string(),
        )
    }

    /// Use Mixtral 8x22B
    pub fn mixtral_8x22b(api_key: String) -> Self {
        Self::new(
            api_key,
            "mistralai/Mixtral-8x22B-Instruct-v0.1".to_string(),
        )
    }

    /// Use Qwen 2.5 72B
    pub fn qwen_72b(api_key: String) -> Self {
        Self::new(api_key, "Qwen/Qwen2.5-72B-Instruct-Turbo".to_string())
    }
}

#[async_trait]
impl LLMProvider for TogetherProvider {
    fn id(&self) -> &str {
        "together"
    }

    fn name(&self) -> &str {
        "Together AI"
    }

    fn model(&self) -> &str {
        self.inner.model()
    }

    async fn health_check(&self) -> bool {
        self.inner.health_check().await
    }

    fn pricing(&self) -> Option<ProviderPricing> {
        ProviderPricing::for_model("together", self.inner.model())
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let mut response = self.inner.chat(request).await?;
        response.provider = "together".to_string();
        Ok(response)
    }

    async fn stream_chat(
        &self,
        request: ChatRequest,
    ) -> Result<mpsc::Receiver<Result<ChatChunk>>> {
        self.inner.stream_chat(request).await
    }

    fn supports_embeddings(&self) -> bool {
        true
    }
}
