//! Groq Provider Implementation
//!
//! Groq provides extremely fast inference for open-source models.

use super::openai::OpenAICompatibleProvider;
use crate::core::llm::cost::{ProviderPricing, TokenUsage};
use crate::core::llm::router::{
    ChatChunk, ChatRequest, ChatResponse, LLMError, LLMProvider, Result,
};
use async_trait::async_trait;
use tokio::sync::mpsc;

const GROQ_BASE_URL: &str = "https://api.groq.com/openai/v1";

/// Groq provider - fast inference for open-source models
pub struct GroqProvider {
    inner: OpenAICompatibleProvider,
}

impl GroqProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            inner: OpenAICompatibleProvider::new(
                "groq".to_string(),
                "Groq".to_string(),
                api_key,
                model,
                8192,
                GROQ_BASE_URL.to_string(),
            ),
        }
    }

    /// Use Llama 3.3 70B
    pub fn llama_70b(api_key: String) -> Self {
        Self::new(api_key, "llama-3.3-70b-versatile".to_string())
    }

    /// Use Llama 3.1 8B (fastest)
    pub fn llama_8b(api_key: String) -> Self {
        Self::new(api_key, "llama-3.1-8b-instant".to_string())
    }

    /// Use Mixtral 8x7B
    pub fn mixtral(api_key: String) -> Self {
        Self::new(api_key, "mixtral-8x7b-32768".to_string())
    }

    /// Use Gemma 2 9B
    pub fn gemma(api_key: String) -> Self {
        Self::new(api_key, "gemma2-9b-it".to_string())
    }
}

#[async_trait]
impl LLMProvider for GroqProvider {
    fn id(&self) -> &str {
        "groq"
    }

    fn name(&self) -> &str {
        "Groq"
    }

    fn model(&self) -> &str {
        self.inner.model()
    }

    async fn health_check(&self) -> bool {
        self.inner.health_check().await
    }

    fn pricing(&self) -> Option<ProviderPricing> {
        ProviderPricing::for_model("groq", self.inner.model())
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let mut response = self.inner.chat(request).await?;
        response.provider = "groq".to_string();
        Ok(response)
    }

    async fn stream_chat(
        &self,
        request: ChatRequest,
    ) -> Result<mpsc::Receiver<Result<ChatChunk>>> {
        self.inner.stream_chat(request).await
    }
}
