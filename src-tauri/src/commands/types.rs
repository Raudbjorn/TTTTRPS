//! Shared Request/Response Types for Tauri Commands
//!
//! Common DTOs used across multiple command modules.

use serde::{Deserialize, Serialize};

// ============================================================================
// Chat Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatRequestPayload {
    pub message: String,
    pub system_prompt: Option<String>,
    pub personality_id: Option<String>,
    pub context: Option<Vec<String>>,
    /// Enable RAG mode to route through Meilisearch Chat
    #[serde(default)]
    pub use_rag: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponsePayload {
    pub content: String,
    pub model: String,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}

// ============================================================================
// LLM Settings Types
// ============================================================================

/// LLM Settings for configuration.
/// Note: Custom Debug impl to avoid exposing api_key in logs.
#[derive(Serialize, Deserialize)]
pub struct LLMSettings {
    pub provider: String,
    pub api_key: Option<String>,
    pub host: Option<String>,
    pub model: String,
    pub embedding_model: Option<String>,
    pub storage_backend: Option<String>,
}

impl std::fmt::Debug for LLMSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LLMSettings")
            .field("provider", &self.provider)
            .field("api_key", &self.api_key.as_ref().map(|_| "<REDACTED>"))
            .field("host", &self.host)
            .field("model", &self.model)
            .field("embedding_model", &self.embedding_model)
            .field("storage_backend", &self.storage_backend)
            .finish()
    }
}

/// Helper to create LLMSettings with common defaults
fn make_llm_settings(provider: &str, model: &str, has_api_key: bool, host: Option<&str>) -> LLMSettings {
    LLMSettings {
        provider: provider.to_string(),
        api_key: if has_api_key { Some("********".to_string()) } else { None },
        host: host.map(String::from),
        model: model.to_string(),
        embedding_model: None,
        storage_backend: None,
    }
}

impl From<&crate::core::llm::LLMConfig> for LLMSettings {
    fn from(config: &crate::core::llm::LLMConfig) -> Self {
        use crate::core::llm::LLMConfig;

        match config {
            LLMConfig::Ollama { host, model } => make_llm_settings("ollama", model, false, Some(host)),
            LLMConfig::Claude { model, storage_backend, .. } => {
                let mut settings = make_llm_settings("claude", model, true, None);
                settings.storage_backend = Some(storage_backend.clone());
                settings
            },
            LLMConfig::Google { model, .. } => make_llm_settings("google", model, true, None),
            LLMConfig::OpenAI { model, .. } => make_llm_settings("openai", model, true, None),
            LLMConfig::OpenRouter { model, .. } => make_llm_settings("openrouter", model, true, None),
            LLMConfig::Mistral { model, .. } => make_llm_settings("mistral", model, true, None),
            LLMConfig::Groq { model, .. } => make_llm_settings("groq", model, true, None),
            LLMConfig::Together { model, .. } => make_llm_settings("together", model, true, None),
            LLMConfig::Cohere { model, .. } => make_llm_settings("cohere", model, true, None),
            LLMConfig::DeepSeek { model, .. } => make_llm_settings("deepseek", model, true, None),
            LLMConfig::Gemini { model, .. } => make_llm_settings("gemini", model, false, None),
            LLMConfig::Meilisearch { host, model, .. } => make_llm_settings("meilisearch", model, false, Some(host)),
            LLMConfig::Copilot { model, .. } => make_llm_settings("copilot", model, false, None),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthStatus {
    pub provider: String,
    pub healthy: bool,
    pub message: String,
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Helper to serialize an enum value to its string representation
pub fn serialize_enum_to_string<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .map(|s| s.trim_matches('"').to_string())
        .unwrap_or_default()
}
