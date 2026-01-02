use crate::core::llm::providers::ProviderConfig;
use crate::core::llm::router::LLMProvider;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use reqwest::Client;

// Re-export ProviderConfig as LLMConfig
pub type LLMConfig = ProviderConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub context_length: Option<u32>,
}

impl From<OllamaModel> for ModelInfo {
    fn from(m: OllamaModel) -> Self {
        Self {
            id: m.name.clone(),
            name: m.name,
            description: Some(format!("Size: {}", m.size)),
            context_length: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    pub size: String,
    pub modified_at: String,
    pub digest: String,
}

pub struct LLMClient {
    config: LLMConfig,
    provider: Arc<dyn LLMProvider>,
}

impl LLMClient {
    pub fn new(config: LLMConfig) -> Self {
        let provider = config.create_provider();
        Self { config, provider }
    }

    pub fn provider_name(&self) -> &str {
        self.config.provider_id()
    }

    pub async fn health_check(&self) -> Result<bool, String> {
        Ok(self.provider.health_check().await)
    }

    pub async fn chat(&self, request: crate::core::llm::router::ChatRequest) -> crate::core::llm::router::Result<crate::core::llm::router::ChatResponse> {
        self.provider.chat(request).await
    }

    pub async fn stream_chat(&self, request: crate::core::llm::router::ChatRequest) -> crate::core::llm::router::Result<tokio::sync::mpsc::Receiver<crate::core::llm::router::Result<crate::core::llm::router::ChatChunk>>> {
         self.provider.stream_chat(request).await
    }

    // Static methods for listing models
    pub async fn list_ollama_models(host: &str) -> Result<Vec<OllamaModel>, String> {
        let url = format!("{}/api/tags", host);
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| e.to_string())?;

        let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("Failed to list models: {}", resp.status()));
        }

        #[derive(Deserialize)]
        struct OllamaTags {
            models: Vec<OllamaModelInternal>,
        }

        #[derive(Deserialize)]
        struct OllamaModelInternal {
            name: String,
            size: u64,
            modified_at: String,
            digest: String,
        }

        let tags: OllamaTags = resp.json().await.map_err(|e| e.to_string())?;

        Ok(tags.models.into_iter().map(|m| OllamaModel {
            name: m.name,
            size: format_size(m.size),
            modified_at: m.modified_at,
            digest: m.digest,
        }).collect())
    }

    pub async fn list_claude_models(_api_key: &str) -> Result<Vec<ModelInfo>, String> {
        // Mock implementation or fallback
        Ok(get_fallback_models("claude"))
    }

    pub async fn list_openai_models(_api_key: &str, _org_id: Option<String>) -> Result<Vec<ModelInfo>, String> {
        Ok(get_fallback_models("openai"))
    }

    pub async fn fetch_openai_models_from_github() -> Result<Vec<ModelInfo>, String> {
        Ok(vec![])
    }

    pub async fn list_gemini_models(_api_key: &str) -> Result<Vec<ModelInfo>, String> {
        Ok(get_fallback_models("gemini"))
    }
}

fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.1} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1} MB", size as f64 / MB as f64)
    } else {
        format!("{} B", size)
    }
}

// Fallback models helper
pub fn get_fallback_models(provider: &str) -> Vec<ModelInfo> {
    match provider {
        "claude" => vec![
            ModelInfo { id: "claude-3-opus-20240229".into(), name: "Claude 3 Opus".into(), description: None, context_length: Some(200000) },
            ModelInfo { id: "claude-3-sonnet-20240229".into(), name: "Claude 3 Sonnet".into(), description: None, context_length: Some(200000) },
            ModelInfo { id: "claude-3-haiku-20240307".into(), name: "Claude 3 Haiku".into(), description: None, context_length: Some(200000) },
        ],
        "openai" => vec![
            ModelInfo { id: "gpt-4-turbo".into(), name: "GPT-4 Turbo".into(), description: None, context_length: Some(128000) },
            ModelInfo { id: "gpt-4".into(), name: "GPT-4".into(), description: None, context_length: Some(8192) },
            ModelInfo { id: "gpt-3.5-turbo".into(), name: "GPT-3.5 Turbo".into(), description: None, context_length: Some(16385) },
        ],
        "gemini" => vec![
            ModelInfo { id: "gemini-pro".into(), name: "Gemini Pro".into(), description: None, context_length: Some(32000) },
            ModelInfo { id: "gemini-1.5-pro".into(), name: "Gemini 1.5 Pro".into(), description: None, context_length: Some(1000000) },
        ],
        _ => vec![]
    }
}

pub fn get_extended_fallback_models(provider: &str) -> Vec<ModelInfo> {
    get_fallback_models(provider)
}

pub async fn fetch_openrouter_models() -> Result<Vec<ModelInfo>, String> {
     Ok(vec![])
}

pub async fn fetch_litellm_models_for_provider(_provider: &str) -> Result<Vec<ModelInfo>, String> {
    Ok(vec![])
}
