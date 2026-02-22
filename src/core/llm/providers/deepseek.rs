//! DeepSeek Provider Implementation
//!
//! DeepSeek provides extremely cost-effective models including DeepSeek Coder.

use super::openai::OpenAICompatibleProvider;
use crate::core::llm::cost::ProviderPricing;
use crate::core::llm::router::{
    ChatChunk, ChatRequest, ChatResponse, LLMProvider, Result,
};
use async_trait::async_trait;
use tokio::sync::mpsc;

const DEEPSEEK_BASE_URL: &str = "https://api.deepseek.com/v1";

/// DeepSeek provider - cost-effective models
pub struct DeepSeekProvider {
    inner: OpenAICompatibleProvider,
}

impl DeepSeekProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            inner: OpenAICompatibleProvider::new(
                "deepseek".to_string(),
                "DeepSeek".to_string(),
                api_key,
                model,
                4096,
                DEEPSEEK_BASE_URL.to_string(),
            ),
        }
    }

    /// Use DeepSeek Chat (general purpose)
    pub fn chat(api_key: String) -> Self {
        Self::new(api_key, "deepseek-chat".to_string())
    }

    /// Use DeepSeek Coder (code-focused)
    pub fn coder(api_key: String) -> Self {
        Self::new(api_key, "deepseek-coder".to_string())
    }

    /// Use DeepSeek Reasoner (R1, reasoning-focused)
    pub fn reasoner(api_key: String) -> Self {
        Self::new(api_key, "deepseek-reasoner".to_string())
    }
}

#[async_trait]
impl LLMProvider for DeepSeekProvider {
    fn id(&self) -> &str {
        "deepseek"
    }

    fn name(&self) -> &str {
        "DeepSeek"
    }

    fn model(&self) -> &str {
        self.inner.model()
    }

    async fn health_check(&self) -> bool {
        // DeepSeek API keys start with "sk-"
        !self.inner.model().is_empty()
    }

    fn pricing(&self) -> Option<ProviderPricing> {
        ProviderPricing::for_model("deepseek", self.inner.model())
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let mut response = self.inner.chat(request).await?;
        response.provider = "deepseek".to_string();
        Ok(response)
    }

    async fn stream_chat(
        &self,
        request: ChatRequest,
    ) -> Result<mpsc::Receiver<Result<ChatChunk>>> {
        self.inner.stream_chat(request).await
    }
}
