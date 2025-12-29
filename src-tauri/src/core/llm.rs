//! LLM Client Module
//!
//! Provides unified interface for Claude, Gemini, and Ollama LLM providers.
//! Supports both chat completion and embedding generation.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum LLMError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("API error: {status} - {message}")]
    ApiError { status: u16, message: String },

    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("Rate limited: retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Provider not configured: {0}")]
    NotConfigured(String),

    #[error("Embedding not supported for provider: {0}")]
    EmbeddingNotSupported(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, LLMError>;

// ============================================================================
// Configuration Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "lowercase")]
pub enum LLMConfig {
    Ollama {
        host: String,
        model: String,
        embedding_model: Option<String>,
    },
    Claude {
        api_key: String,
        model: String,
        #[serde(default = "default_claude_max_tokens")]
        max_tokens: u32,
    },
    Gemini {
        api_key: String,
        model: String,
    },
    OpenAI {
        api_key: String,
        model: String,
        #[serde(default = "default_openai_max_tokens")]
        max_tokens: u32,
        #[serde(default)]
        organization_id: Option<String>,
        #[serde(default = "default_openai_base_url")]
        base_url: String,
    },
    /// OpenRouter - aggregates 400+ models from multiple providers
    OpenRouter {
        api_key: String,
        model: String,
    },
    /// Mistral AI
    Mistral {
        api_key: String,
        model: String,
    },
    /// Groq - fast inference
    Groq {
        api_key: String,
        model: String,
    },
    /// Together AI - open source models
    Together {
        api_key: String,
        model: String,
    },
    /// Cohere
    Cohere {
        api_key: String,
        model: String,
    },
    /// DeepSeek
    DeepSeek {
        api_key: String,
        model: String,
    },
}

fn default_openai_max_tokens() -> u32 {
    4096
}

fn default_openai_base_url() -> String {
    "https://api.openai.com/v1".to_string()
}

fn default_claude_max_tokens() -> u32 {
    4096
}

impl Default for LLMConfig {
    fn default() -> Self {
        LLMConfig::Ollama {
            host: "http://localhost:11434".to_string(),
            model: "llama3.2".to_string(),
            embedding_model: Some("nomic-embed-text".to_string()),
        }
    }
}

// ============================================================================
// Message Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub model: String,
    pub usage: Option<TokenUsage>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct EmbeddingResponse {
    pub embedding: Vec<f32>,
    pub model: String,
    pub dimensions: usize,
}

/// Information about an Ollama model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    pub size: Option<String>,
    pub parameter_size: Option<String>,
}

/// Generic model info for any provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

/// Hardcoded fallback models for each provider
pub fn get_fallback_models(provider: &str) -> Vec<ModelInfo> {
    match provider {
        "claude" => vec![
            ModelInfo { id: "claude-sonnet-4-20250514".to_string(), name: "Claude Sonnet 4".to_string(), description: Some("Latest balanced model".to_string()) },
            ModelInfo { id: "claude-3-5-sonnet-20241022".to_string(), name: "Claude 3.5 Sonnet".to_string(), description: Some("Best for most tasks".to_string()) },
            ModelInfo { id: "claude-3-5-haiku-20241022".to_string(), name: "Claude 3.5 Haiku".to_string(), description: Some("Fast and efficient".to_string()) },
            ModelInfo { id: "claude-3-opus-20240229".to_string(), name: "Claude 3 Opus".to_string(), description: Some("Most capable".to_string()) },
        ],
        "openai" => vec![
            ModelInfo { id: "gpt-4o".to_string(), name: "GPT-4o".to_string(), description: Some("Latest multimodal".to_string()) },
            ModelInfo { id: "gpt-4o-mini".to_string(), name: "GPT-4o Mini".to_string(), description: Some("Fast and affordable".to_string()) },
            ModelInfo { id: "gpt-4-turbo".to_string(), name: "GPT-4 Turbo".to_string(), description: Some("High capability".to_string()) },
            ModelInfo { id: "gpt-3.5-turbo".to_string(), name: "GPT-3.5 Turbo".to_string(), description: Some("Fast, legacy".to_string()) },
            ModelInfo { id: "o1-preview".to_string(), name: "o1 Preview".to_string(), description: Some("Reasoning model".to_string()) },
            ModelInfo { id: "o1-mini".to_string(), name: "o1 Mini".to_string(), description: Some("Fast reasoning".to_string()) },
        ],
        "gemini" => vec![
            ModelInfo { id: "gemini-2.0-flash-exp".to_string(), name: "Gemini 2.0 Flash".to_string(), description: Some("Latest experimental".to_string()) },
            ModelInfo { id: "gemini-1.5-pro".to_string(), name: "Gemini 1.5 Pro".to_string(), description: Some("Best quality".to_string()) },
            ModelInfo { id: "gemini-1.5-flash".to_string(), name: "Gemini 1.5 Flash".to_string(), description: Some("Fast and efficient".to_string()) },
            ModelInfo { id: "gemini-1.0-pro".to_string(), name: "Gemini 1.0 Pro".to_string(), description: Some("Stable".to_string()) },
        ],
        _ => vec![],
    }
}

// ============================================================================
// LLM Client
// ============================================================================

pub struct LLMClient {
    client: Client,
    config: LLMConfig,
}

impl LLMClient {
    const TIMEOUT_SECS: u64 = 120;

    pub fn new(config: LLMConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(Self::TIMEOUT_SECS))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, config }
    }

    /// Get the provider name
    pub fn provider_name(&self) -> &'static str {
        match &self.config {
            LLMConfig::Ollama { .. } => "ollama",
            LLMConfig::Claude { .. } => "claude",
            LLMConfig::Gemini { .. } => "gemini",
            LLMConfig::OpenAI { .. } => "openai",
            LLMConfig::OpenRouter { .. } => "openrouter",
            LLMConfig::Mistral { .. } => "mistral",
            LLMConfig::Groq { .. } => "groq",
            LLMConfig::Together { .. } => "together",
            LLMConfig::Cohere { .. } => "cohere",
            LLMConfig::DeepSeek { .. } => "deepseek",
        }
    }

    /// Send a chat completion request
    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        match &self.config {
            LLMConfig::Ollama { host, model, .. } => {
                self.ollama_chat(host, model, request).await
            }
            LLMConfig::Claude { api_key, model, max_tokens } => {
                self.claude_chat(api_key, model, *max_tokens, request).await
            }
            LLMConfig::Gemini { api_key, model } => {
                self.gemini_chat(api_key, model, request).await
            }
            LLMConfig::OpenAI { api_key, model, max_tokens, organization_id, base_url } => {
                self.openai_chat(api_key, model, *max_tokens, organization_id.as_deref(), base_url, request).await
            }
            // OpenAI-compatible providers
            LLMConfig::OpenRouter { api_key, model } => {
                self.openai_chat(api_key, model, 4096, None, "https://openrouter.ai/api/v1", request).await
            }
            LLMConfig::Mistral { api_key, model } => {
                self.openai_chat(api_key, model, 4096, None, "https://api.mistral.ai/v1", request).await
            }
            LLMConfig::Groq { api_key, model } => {
                self.openai_chat(api_key, model, 4096, None, "https://api.groq.com/openai/v1", request).await
            }
            LLMConfig::Together { api_key, model } => {
                self.openai_chat(api_key, model, 4096, None, "https://api.together.xyz/v1", request).await
            }
            LLMConfig::Cohere { api_key, model } => {
                self.cohere_chat(api_key, model, request).await
            }
            LLMConfig::DeepSeek { api_key, model } => {
                self.openai_chat(api_key, model, 4096, None, "https://api.deepseek.com/v1", request).await
            }
        }
    }

    /// Generate embeddings for text
    pub async fn embed(&self, text: &str) -> Result<EmbeddingResponse> {
        match &self.config {
            LLMConfig::Ollama { host, embedding_model, .. } => {
                let model = embedding_model.as_deref().unwrap_or("nomic-embed-text");
                self.ollama_embed(host, model, text).await
            }
            LLMConfig::Claude { .. } => {
                Err(LLMError::EmbeddingNotSupported(
                    "Claude requires Voyage API for embeddings".to_string()
                ))
            }
            LLMConfig::Gemini { api_key, .. } => {
                self.gemini_embed(api_key, text).await
            }
            LLMConfig::OpenAI { api_key, base_url, .. } => {
                self.openai_embed(api_key, base_url, text).await
            }
            LLMConfig::Together { api_key, .. } => {
                self.openai_embed(api_key, "https://api.together.xyz/v1", text).await
            }
            LLMConfig::Cohere { api_key, .. } => {
                self.cohere_embed(api_key, text).await
            }
            // These providers don't have embedding APIs
            LLMConfig::OpenRouter { .. } | LLMConfig::Mistral { .. } |
            LLMConfig::Groq { .. } | LLMConfig::DeepSeek { .. } => {
                Err(LLMError::EmbeddingNotSupported(
                    "Provider does not support embeddings".to_string()
                ))
            }
        }
    }

    /// Check if the provider is available/healthy
    pub async fn health_check(&self) -> Result<bool> {
        match &self.config {
            LLMConfig::Ollama { host, .. } => {
                let url = format!("{}/api/tags", host);
                match self.client.get(&url).send().await {
                    Ok(resp) => Ok(resp.status().is_success()),
                    Err(_) => Ok(false),
                }
            }
            LLMConfig::Claude { api_key, .. } => {
                Ok(api_key.starts_with("sk-ant-"))
            }
            LLMConfig::Gemini { api_key, .. } => {
                Ok(api_key.starts_with("AIza"))
            }
            LLMConfig::OpenAI { api_key, base_url, .. } => {
                let url = format!("{}/models", base_url);
                match self.client.get(&url)
                    .header("Authorization", format!("Bearer {}", api_key))
                    .send()
                    .await
                {
                    Ok(resp) => Ok(resp.status().is_success()),
                    Err(_) => Ok(false),
                }
            }
            // OpenAI-compatible providers - check models endpoint
            LLMConfig::OpenRouter { api_key, .. } => {
                let url = "https://openrouter.ai/api/v1/models";
                match self.client.get(url)
                    .header("Authorization", format!("Bearer {}", api_key))
                    .send().await
                {
                    Ok(resp) => Ok(resp.status().is_success()),
                    Err(_) => Ok(false),
                }
            }
            LLMConfig::Mistral { api_key, .. } => {
                let url = "https://api.mistral.ai/v1/models";
                match self.client.get(url)
                    .header("Authorization", format!("Bearer {}", api_key))
                    .send().await
                {
                    Ok(resp) => Ok(resp.status().is_success()),
                    Err(_) => Ok(false),
                }
            }
            LLMConfig::Groq { api_key, .. } => {
                let url = "https://api.groq.com/openai/v1/models";
                match self.client.get(url)
                    .header("Authorization", format!("Bearer {}", api_key))
                    .send().await
                {
                    Ok(resp) => Ok(resp.status().is_success()),
                    Err(_) => Ok(false),
                }
            }
            LLMConfig::Together { api_key, .. } => {
                let url = "https://api.together.xyz/v1/models";
                match self.client.get(url)
                    .header("Authorization", format!("Bearer {}", api_key))
                    .send().await
                {
                    Ok(resp) => Ok(resp.status().is_success()),
                    Err(_) => Ok(false),
                }
            }
            LLMConfig::Cohere { api_key, .. } => {
                Ok(!api_key.is_empty())
            }
            LLMConfig::DeepSeek { api_key, .. } => {
                Ok(api_key.starts_with("sk-"))
            }
        }
    }

    // ========================================================================
    // Ollama Model Listing
    // ========================================================================

    /// List available models from Ollama
    pub async fn list_ollama_models(host: &str) -> Result<Vec<OllamaModel>> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| LLMError::InvalidResponse(e.to_string()))?;

        let url = format!("{}/api/tags", host);
        let response = client.get(&url).send().await
            .map_err(|e| LLMError::InvalidResponse(format!("Failed to connect to Ollama: {}", e)))?;

        if !response.status().is_success() {
            return Err(LLMError::ApiError {
                status: response.status().as_u16(),
                message: format!("Ollama returned status: {}", response.status())
            });
        }

        #[derive(Deserialize)]
        struct OllamaTagsResponse {
            models: Vec<OllamaModelInfo>,
        }

        #[derive(Deserialize)]
        struct OllamaModelInfo {
            name: String,
            size: Option<u64>,
            #[serde(default)]
            details: Option<OllamaModelDetails>,
        }

        #[derive(Deserialize, Default)]
        struct OllamaModelDetails {
            family: Option<String>,
            parameter_size: Option<String>,
        }

        let tags: OllamaTagsResponse = response.json().await
            .map_err(|e| LLMError::InvalidResponse(format!("Failed to parse Ollama response: {}", e)))?;

        let models = tags.models.into_iter().map(|m| {
            let size_str = m.size.map(|s| {
                if s > 1_000_000_000 {
                    format!("{:.1}GB", s as f64 / 1_000_000_000.0)
                } else {
                    format!("{:.0}MB", s as f64 / 1_000_000.0)
                }
            });
            let param_size = m.details.and_then(|d| d.parameter_size);

            OllamaModel {
                name: m.name,
                size: size_str,
                parameter_size: param_size,
            }
        }).collect();

        Ok(models)
    }

    /// List available models from Claude/Anthropic API
    pub async fn list_claude_models(api_key: &str) -> Result<Vec<ModelInfo>> {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| LLMError::InvalidResponse(e.to_string()))?;

        let response = client
            .get("https://api.anthropic.com/v1/models")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .send()
            .await
            .map_err(|e| LLMError::InvalidResponse(format!("Failed to connect: {}", e)))?;

        if !response.status().is_success() {
            return Err(LLMError::ApiError {
                status: response.status().as_u16(),
                message: "Failed to fetch models".to_string(),
            });
        }

        #[derive(Deserialize)]
        struct ClaudeModelsResponse {
            data: Vec<ClaudeModelInfo>,
        }

        #[derive(Deserialize)]
        struct ClaudeModelInfo {
            id: String,
            display_name: Option<String>,
        }

        let resp: ClaudeModelsResponse = response.json().await
            .map_err(|e| LLMError::InvalidResponse(e.to_string()))?;

        let models = resp.data.into_iter()
            .filter(|m| m.id.contains("claude"))
            .map(|m| ModelInfo {
                name: m.display_name.unwrap_or_else(|| m.id.clone()),
                id: m.id,
                description: None,
            })
            .collect();

        Ok(models)
    }

    /// List available models from OpenAI API
    pub async fn list_openai_models(api_key: &str, base_url: Option<&str>) -> Result<Vec<ModelInfo>> {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| LLMError::InvalidResponse(e.to_string()))?;

        let url = format!("{}/models", base_url.unwrap_or("https://api.openai.com/v1"));

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| LLMError::InvalidResponse(format!("Failed to connect: {}", e)))?;

        if !response.status().is_success() {
            return Err(LLMError::ApiError {
                status: response.status().as_u16(),
                message: "Failed to fetch models".to_string(),
            });
        }

        #[derive(Deserialize)]
        struct OpenAIModelsResponse {
            data: Vec<OpenAIModelInfo>,
        }

        #[derive(Deserialize)]
        struct OpenAIModelInfo {
            id: String,
        }

        let resp: OpenAIModelsResponse = response.json().await
            .map_err(|e| LLMError::InvalidResponse(e.to_string()))?;

        // Filter to chat models only
        let chat_prefixes = ["gpt-4", "gpt-3.5", "gpt-5", "o1", "o3", "o4", "chatgpt"];
        let models = resp.data.into_iter()
            .filter(|m| chat_prefixes.iter().any(|p| m.id.starts_with(p)))
            .map(|m| ModelInfo {
                name: m.id.clone(),
                id: m.id,
                description: None,
            })
            .collect();

        Ok(models)
    }

    /// Fetch OpenAI models list from community-maintained GitHub repo
    /// This is used as a fallback when no API key is provided
    pub async fn fetch_openai_models_from_github() -> Result<Vec<ModelInfo>> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| LLMError::InvalidResponse(e.to_string()))?;

        let url = "https://github.com/c0des1ayr/openai-models-list/releases/download/continuous/models.json";

        let response = client
            .get(url)
            .header("User-Agent", "TTRPG-Assistant")
            .send()
            .await
            .map_err(|e| LLMError::InvalidResponse(format!("Failed to fetch models list: {}", e)))?;

        if !response.status().is_success() {
            return Err(LLMError::ApiError {
                status: response.status().as_u16(),
                message: "Failed to fetch models from GitHub".to_string(),
            });
        }

        #[derive(Deserialize)]
        struct GithubModelsResponse {
            models: Vec<String>,
        }

        let resp: GithubModelsResponse = response.json().await
            .map_err(|e| LLMError::InvalidResponse(format!("Failed to parse models: {}", e)))?;

        // Filter to chat/completion models only (exclude embeddings, tts, whisper, etc.)
        let chat_prefixes = ["gpt-3.5", "gpt-4", "gpt-5", "o1", "o3", "o4", "chatgpt"];
        let models: Vec<ModelInfo> = resp.models.into_iter()
            .filter(|m| chat_prefixes.iter().any(|p| m.starts_with(p)))
            .filter(|m| !m.contains("transcribe") && !m.contains("tts") && !m.contains("audio") && !m.contains("image"))
            .map(|m| ModelInfo {
                name: m.clone(),
                id: m,
                description: None,
            })
            .collect();

        Ok(models)
    }

    /// List available models from Gemini/Google API
    pub async fn list_gemini_models(api_key: &str) -> Result<Vec<ModelInfo>> {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| LLMError::InvalidResponse(e.to_string()))?;

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models?key={}",
            api_key
        );

        let response = client.get(&url).send().await
            .map_err(|e| LLMError::InvalidResponse(format!("Failed to connect: {}", e)))?;

        if !response.status().is_success() {
            return Err(LLMError::ApiError {
                status: response.status().as_u16(),
                message: "Failed to fetch models".to_string(),
            });
        }

        #[derive(Deserialize)]
        struct GeminiModelsResponse {
            models: Vec<GeminiModelInfo>,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct GeminiModelInfo {
            name: String,
            display_name: Option<String>,
            description: Option<String>,
        }

        let resp: GeminiModelsResponse = response.json().await
            .map_err(|e| LLMError::InvalidResponse(e.to_string()))?;

        // Filter to generative models and extract model ID from full name
        let models = resp.models.into_iter()
            .filter(|m| m.name.contains("gemini"))
            .map(|m| {
                let id = m.name.strip_prefix("models/").unwrap_or(&m.name).to_string();
                ModelInfo {
                    name: m.display_name.unwrap_or_else(|| id.clone()),
                    id,
                    description: m.description,
                }
            })
            .collect();

        Ok(models)
    }

    // ========================================================================
    // Ollama Implementation
    // ========================================================================

    async fn ollama_chat(&self, host: &str, model: &str, request: ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/api/chat", host);

        // Build messages with system prompt
        let mut messages: Vec<serde_json::Value> = Vec::new();

        if let Some(system) = &request.system_prompt {
            messages.push(serde_json::json!({
                "role": "system",
                "content": system
            }));
        }

        for msg in &request.messages {
            messages.push(serde_json::json!({
                "role": match msg.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                },
                "content": msg.content
            }));
        }

        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": false,
            "options": {
                "temperature": request.temperature.unwrap_or(0.7)
            }
        });

        let resp = self.client.post(&url)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(LLMError::ApiError { status, message: text });
        }

        let json: serde_json::Value = resp.json().await?;

        let content = json["message"]["content"]
            .as_str()
            .ok_or_else(|| LLMError::InvalidResponse("Missing content".to_string()))?
            .to_string();

        Ok(ChatResponse {
            content,
            model: model.to_string(),
            usage: Some(TokenUsage {
                input_tokens: json["prompt_eval_count"].as_u64().unwrap_or(0) as u32,
                output_tokens: json["eval_count"].as_u64().unwrap_or(0) as u32,
            }),
            finish_reason: Some("stop".to_string()),
        })
    }

    async fn ollama_embed(&self, host: &str, model: &str, text: &str) -> Result<EmbeddingResponse> {
        let url = format!("{}/api/embeddings", host);

        let body = serde_json::json!({
            "model": model,
            "prompt": text
        });

        let resp = self.client.post(&url)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(LLMError::ApiError { status, message: text });
        }

        let json: serde_json::Value = resp.json().await?;

        let embedding: Vec<f32> = json["embedding"]
            .as_array()
            .ok_or_else(|| LLMError::InvalidResponse("Missing embedding".to_string()))?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        let dimensions = embedding.len();

        Ok(EmbeddingResponse {
            embedding,
            model: model.to_string(),
            dimensions,
        })
    }

    // ========================================================================
    // Claude Implementation
    // ========================================================================

    async fn claude_chat(
        &self,
        api_key: &str,
        model: &str,
        max_tokens: u32,
        request: ChatRequest
    ) -> Result<ChatResponse> {
        const API_URL: &str = "https://api.anthropic.com/v1/messages";
        const API_VERSION: &str = "2023-06-01";

        // Build messages (Claude has separate system parameter)
        let messages: Vec<serde_json::Value> = request.messages.iter()
            .filter(|m| m.role != MessageRole::System)
            .map(|m| serde_json::json!({
                "role": match m.role {
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::System => "user", // Shouldn't happen due to filter
                },
                "content": m.content
            }))
            .collect();

        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(max_tokens)
        });

        // Add system prompt if present
        if let Some(system) = &request.system_prompt {
            body["system"] = serde_json::Value::String(system.clone());
        }

        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::Value::Number(
                serde_json::Number::from_f64(temp as f64).unwrap()
            );
        }

        let resp = self.client.post(API_URL)
            .header("x-api-key", api_key)
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = resp.headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok())
                .unwrap_or(60);
            return Err(LLMError::RateLimited { retry_after_secs: retry_after });
        }

        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(LLMError::AuthError("Invalid API key".to_string()));
        }

        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(LLMError::ApiError { status: status.as_u16(), message: text });
        }

        let json: serde_json::Value = resp.json().await?;

        // Claude returns content as an array
        let content = json["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|c| c["text"].as_str())
            .ok_or_else(|| LLMError::InvalidResponse("Missing content".to_string()))?
            .to_string();

        let usage = json["usage"].as_object().map(|u| TokenUsage {
            input_tokens: u["input_tokens"].as_u64().unwrap_or(0) as u32,
            output_tokens: u["output_tokens"].as_u64().unwrap_or(0) as u32,
        });

        Ok(ChatResponse {
            content,
            model: json["model"].as_str().unwrap_or(model).to_string(),
            usage,
            finish_reason: json["stop_reason"].as_str().map(|s| s.to_string()),
        })
    }

    // ========================================================================
    // Gemini Implementation
    // ========================================================================

    async fn gemini_chat(&self, api_key: &str, model: &str, request: ChatRequest) -> Result<ChatResponse> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, api_key
        );

        // Build contents array
        let mut contents: Vec<serde_json::Value> = Vec::new();

        for msg in &request.messages {
            let role = match msg.role {
                MessageRole::User => "user",
                MessageRole::Assistant => "model",
                MessageRole::System => continue, // Handle separately
            };

            contents.push(serde_json::json!({
                "role": role,
                "parts": [{ "text": msg.content }]
            }));
        }

        let mut body = serde_json::json!({
            "contents": contents
        });

        // Add system instruction if present
        if let Some(system) = &request.system_prompt {
            body["systemInstruction"] = serde_json::json!({
                "parts": [{ "text": system }]
            });
        }

        // Add generation config
        let mut gen_config = serde_json::Map::new();
        if let Some(temp) = request.temperature {
            gen_config.insert(
                "temperature".to_string(),
                serde_json::Value::Number(serde_json::Number::from_f64(temp as f64).unwrap())
            );
        }
        if let Some(max) = request.max_tokens {
            gen_config.insert(
                "maxOutputTokens".to_string(),
                serde_json::Value::Number(serde_json::Number::from(max))
            );
        }
        if !gen_config.is_empty() {
            body["generationConfig"] = serde_json::Value::Object(gen_config);
        }

        let resp = self.client.post(&url)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();

        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(LLMError::ApiError { status: status.as_u16(), message: text });
        }

        let json: serde_json::Value = resp.json().await?;

        let content = json["candidates"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|c| c["content"]["parts"].as_array())
            .and_then(|parts| parts.first())
            .and_then(|p| p["text"].as_str())
            .ok_or_else(|| LLMError::InvalidResponse("Missing content".to_string()))?
            .to_string();

        let usage = json["usageMetadata"].as_object().map(|u| TokenUsage {
            input_tokens: u["promptTokenCount"].as_u64().unwrap_or(0) as u32,
            output_tokens: u["candidatesTokenCount"].as_u64().unwrap_or(0) as u32,
        });

        Ok(ChatResponse {
            content,
            model: model.to_string(),
            usage,
            finish_reason: json["candidates"]
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|c| c["finishReason"].as_str())
                .map(|s| s.to_string()),
        })
    }

    async fn gemini_embed(&self, api_key: &str, text: &str) -> Result<EmbeddingResponse> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/text-embedding-004:embedContent?key={}",
            api_key
        );

        let body = serde_json::json!({
            "model": "models/text-embedding-004",
            "content": {
                "parts": [{ "text": text }]
            }
        });

        let resp = self.client.post(&url)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(LLMError::ApiError { status, message: text });
        }

        let json: serde_json::Value = resp.json().await?;

        let embedding: Vec<f32> = json["embedding"]["values"]
            .as_array()
            .ok_or_else(|| LLMError::InvalidResponse("Missing embedding".to_string()))?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        let dimensions = embedding.len();

        Ok(EmbeddingResponse {
            embedding,
            model: "text-embedding-004".to_string(),
            dimensions,
        })
    }

    // ========================================================================
    // Cohere Implementation
    // ========================================================================

    async fn cohere_chat(&self, api_key: &str, model: &str, request: ChatRequest) -> Result<ChatResponse> {
        let url = "https://api.cohere.ai/v1/chat";

        // Cohere uses a different message format
        let mut chat_history: Vec<serde_json::Value> = Vec::new();

        // Add conversation history (excluding the last message which becomes the query)
        for msg in request.messages.iter().take(request.messages.len().saturating_sub(1)) {
            let role = match msg.role {
                MessageRole::System => "SYSTEM",
                MessageRole::User => "USER",
                MessageRole::Assistant => "CHATBOT",
            };
            chat_history.push(serde_json::json!({
                "role": role,
                "message": msg.content
            }));
        }

        // Last message is the current query
        let message = request.messages.last()
            .map(|m| m.content.clone())
            .unwrap_or_default();

        let mut body = serde_json::json!({
            "model": model,
            "message": message,
        });

        if !chat_history.is_empty() {
            body["chat_history"] = serde_json::Value::Array(chat_history);
        }

        if let Some(system) = &request.system_prompt {
            body["preamble"] = serde_json::Value::String(system.clone());
        }

        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        let resp = self.client.post(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(LLMError::ApiError { status: status.as_u16(), message: text });
        }

        let json: serde_json::Value = resp.json().await?;

        let content = json["text"]
            .as_str()
            .ok_or_else(|| LLMError::InvalidResponse("Missing text in response".to_string()))?
            .to_string();

        let usage = json["meta"]["tokens"].as_object().map(|t| TokenUsage {
            input_tokens: t["input_tokens"].as_u64().unwrap_or(0) as u32,
            output_tokens: t["output_tokens"].as_u64().unwrap_or(0) as u32,
        });

        Ok(ChatResponse {
            content,
            model: model.to_string(),
            usage,
            finish_reason: json["finish_reason"].as_str().map(|s| s.to_string()),
        })
    }

    async fn cohere_embed(&self, api_key: &str, text: &str) -> Result<EmbeddingResponse> {
        let url = "https://api.cohere.ai/v1/embed";

        let body = serde_json::json!({
            "texts": [text],
            "model": "embed-english-v3.0",
            "input_type": "search_document"
        });

        let resp = self.client.post(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(LLMError::ApiError { status, message: text });
        }

        let json: serde_json::Value = resp.json().await?;

        let embedding: Vec<f32> = json["embeddings"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|e| e.as_array())
            .ok_or_else(|| LLMError::InvalidResponse("Missing embeddings".to_string()))?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        let dimensions = embedding.len();

        Ok(EmbeddingResponse {
            embedding,
            model: "embed-english-v3.0".to_string(),
            dimensions,
        })
    }

    // ========================================================================
    // OpenAI Implementation
    // ========================================================================

    async fn openai_chat(
        &self,
        api_key: &str,
        model: &str,
        max_tokens: u32,
        organization_id: Option<&str>,
        base_url: &str,
        request: ChatRequest,
    ) -> Result<ChatResponse> {
        let url = format!("{}/chat/completions", base_url);

        // Build messages array
        let mut messages: Vec<serde_json::Value> = Vec::new();

        // Add system prompt as first message if present
        if let Some(system) = &request.system_prompt {
            messages.push(serde_json::json!({
                "role": "system",
                "content": system
            }));
        }

        // Add conversation messages
        for msg in &request.messages {
            messages.push(serde_json::json!({
                "role": match msg.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                },
                "content": msg.content
            }));
        }

        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(max_tokens)
        });

        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::Value::Number(
                serde_json::Number::from_f64(temp as f64).unwrap()
            );
        }

        let mut req_builder = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json");

        if let Some(org_id) = organization_id {
            req_builder = req_builder.header("OpenAI-Organization", org_id);
        }

        let resp = req_builder.json(&body).send().await?;

        let status = resp.status();

        // Handle rate limiting
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = resp.headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok())
                .unwrap_or(60);
            return Err(LLMError::RateLimited { retry_after_secs: retry_after });
        }

        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(LLMError::AuthError("Invalid OpenAI API key".to_string()));
        }

        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(LLMError::ApiError { status: status.as_u16(), message: text });
        }

        let json: serde_json::Value = resp.json().await?;

        // Extract content from choices array
        let content = json["choices"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|c| c["message"]["content"].as_str())
            .ok_or_else(|| LLMError::InvalidResponse("Missing content in response".to_string()))?
            .to_string();

        let usage = json["usage"].as_object().map(|u| TokenUsage {
            input_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            output_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
        });

        let finish_reason = json["choices"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|c| c["finish_reason"].as_str())
            .map(|s| s.to_string());

        Ok(ChatResponse {
            content,
            model: json["model"].as_str().unwrap_or(model).to_string(),
            usage,
            finish_reason,
        })
    }

    async fn openai_embed(&self, api_key: &str, base_url: &str, text: &str) -> Result<EmbeddingResponse> {
        let url = format!("{}/embeddings", base_url);

        let body = serde_json::json!({
            "model": "text-embedding-3-small",
            "input": text
        });

        let resp = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(LLMError::ApiError { status, message: text });
        }

        let json: serde_json::Value = resp.json().await?;

        let embedding: Vec<f32> = json["data"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|d| d["embedding"].as_array())
            .ok_or_else(|| LLMError::InvalidResponse("Missing embedding in response".to_string()))?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        let dimensions = embedding.len();

        Ok(EmbeddingResponse {
            embedding,
            model: "text-embedding-3-small".to_string(),
            dimensions,
        })
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
        }
    }
}

impl ChatRequest {
    pub fn new(messages: Vec<ChatMessage>) -> Self {
        Self {
            messages,
            system_prompt: None,
            temperature: None,
            max_tokens: None,
        }
    }

    pub fn with_system(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    pub fn with_max_tokens(mut self, max: u32) -> Self {
        self.max_tokens = Some(max);
        self
    }
}

// ============================================================================
// External Model Catalog Fetchers (No Auth Required)
// ============================================================================

/// Model info with extended metadata from LiteLLM catalog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedModelInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub description: Option<String>,
    pub context_window: Option<u32>,
    pub input_cost_per_million: Option<f64>,
    pub output_cost_per_million: Option<f64>,
    pub supports_vision: bool,
    pub supports_function_calling: bool,
}

impl From<ExtendedModelInfo> for ModelInfo {
    fn from(e: ExtendedModelInfo) -> Self {
        ModelInfo {
            id: e.id,
            name: e.name,
            description: e.description,
        }
    }
}

/// Fetch comprehensive model catalog from BerriAI/litellm (no auth required)
/// Returns models grouped by provider
pub async fn fetch_litellm_catalog() -> Result<std::collections::HashMap<String, Vec<ExtendedModelInfo>>> {
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| LLMError::InvalidResponse(e.to_string()))?;

    let url = "https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json";

    let response = client
        .get(url)
        .header("User-Agent", "TTRPG-Assistant")
        .send()
        .await
        .map_err(|e| LLMError::InvalidResponse(format!("Failed to fetch LiteLLM catalog: {}", e)))?;

    if !response.status().is_success() {
        return Err(LLMError::ApiError {
            status: response.status().as_u16(),
            message: "Failed to fetch LiteLLM catalog".to_string(),
        });
    }

    #[derive(Deserialize)]
    struct LiteLLMModel {
        litellm_provider: Option<String>,
        max_input_tokens: Option<u32>,
        max_output_tokens: Option<u32>,
        input_cost_per_token: Option<f64>,
        output_cost_per_token: Option<f64>,
        supports_vision: Option<bool>,
        supports_function_calling: Option<bool>,
        mode: Option<String>,
    }

    let models: std::collections::HashMap<String, LiteLLMModel> = response.json().await
        .map_err(|e| LLMError::InvalidResponse(format!("Failed to parse LiteLLM catalog: {}", e)))?;

    // Group by provider and filter to chat models
    let mut grouped: std::collections::HashMap<String, Vec<ExtendedModelInfo>> = std::collections::HashMap::new();

    for (model_id, model) in models {
        // Skip non-chat models
        if let Some(mode) = &model.mode {
            if mode != "chat" && mode != "completion" {
                continue;
            }
        }

        let provider = model.litellm_provider.clone().unwrap_or_else(|| "unknown".to_string());

        // Skip embedding-only, image, audio models
        if model_id.contains("embedding") || model_id.contains("tts") ||
           model_id.contains("whisper") || model_id.contains("dall-e") {
            continue;
        }

        let info = ExtendedModelInfo {
            id: model_id.clone(),
            name: model_id.clone(),
            provider: provider.clone(),
            description: None,
            context_window: model.max_input_tokens,
            input_cost_per_million: model.input_cost_per_token.map(|c| c * 1_000_000.0),
            output_cost_per_million: model.output_cost_per_token.map(|c| c * 1_000_000.0),
            supports_vision: model.supports_vision.unwrap_or(false),
            supports_function_calling: model.supports_function_calling.unwrap_or(false),
        };

        grouped.entry(provider).or_default().push(info);
    }

    Ok(grouped)
}

/// Fetch models from LiteLLM for a specific provider
pub async fn fetch_litellm_models_for_provider(provider: &str) -> Result<Vec<ModelInfo>> {
    let catalog = fetch_litellm_catalog().await?;

    // Map provider names
    let litellm_provider = match provider {
        "claude" | "anthropic" => "anthropic",
        "openai" => "openai",
        "gemini" | "google" => "vertex_ai-language-models",
        "mistral" => "mistral",
        "groq" => "groq",
        "together" => "together_ai",
        "openrouter" => "openrouter",
        "cohere" => "cohere_chat",
        "deepseek" => "deepseek",
        "fireworks" => "fireworks_ai",
        other => other,
    };

    let models = catalog.get(litellm_provider)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(ModelInfo::from)
        .collect();

    Ok(models)
}

/// Fetch models from OpenRouter API (no auth required)
pub async fn fetch_openrouter_models() -> Result<Vec<ExtendedModelInfo>> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| LLMError::InvalidResponse(e.to_string()))?;

    let url = "https://openrouter.ai/api/v1/models";

    let response = client
        .get(url)
        .header("User-Agent", "TTRPG-Assistant")
        .send()
        .await
        .map_err(|e| LLMError::InvalidResponse(format!("Failed to fetch OpenRouter models: {}", e)))?;

    if !response.status().is_success() {
        return Err(LLMError::ApiError {
            status: response.status().as_u16(),
            message: "Failed to fetch OpenRouter models".to_string(),
        });
    }

    #[derive(Deserialize)]
    struct OpenRouterResponse {
        data: Vec<OpenRouterModel>,
    }

    #[derive(Deserialize)]
    struct OpenRouterModel {
        id: String,
        name: String,
        context_length: Option<u32>,
        pricing: Option<OpenRouterPricing>,
        architecture: Option<OpenRouterArch>,
    }

    #[derive(Deserialize)]
    struct OpenRouterPricing {
        prompt: Option<String>,
        completion: Option<String>,
    }

    #[derive(Deserialize)]
    struct OpenRouterArch {
        modality: Option<String>,
    }

    let resp: OpenRouterResponse = response.json().await
        .map_err(|e| LLMError::InvalidResponse(format!("Failed to parse OpenRouter response: {}", e)))?;

    let models = resp.data.into_iter()
        .map(|m| {
            let supports_vision = m.architecture
                .as_ref()
                .and_then(|a| a.modality.as_ref())
                .map(|m| m.contains("image"))
                .unwrap_or(false);

            // Parse pricing (string format like "0.0000025")
            let input_cost = m.pricing.as_ref()
                .and_then(|p| p.prompt.as_ref())
                .and_then(|s| s.parse::<f64>().ok())
                .map(|c| c * 1_000_000.0);

            let output_cost = m.pricing.as_ref()
                .and_then(|p| p.completion.as_ref())
                .and_then(|s| s.parse::<f64>().ok())
                .map(|c| c * 1_000_000.0);

            // Extract provider from model ID (e.g., "openai/gpt-4" -> "openai")
            let provider = m.id.split('/').next().unwrap_or("unknown").to_string();

            ExtendedModelInfo {
                id: m.id,
                name: m.name,
                provider,
                description: None,
                context_window: m.context_length,
                input_cost_per_million: input_cost,
                output_cost_per_million: output_cost,
                supports_vision,
                supports_function_calling: true, // OpenRouter generally supports this
            }
        })
        .collect();

    Ok(models)
}

/// Get fallback models for new providers
pub fn get_extended_fallback_models(provider: &str) -> Vec<ModelInfo> {
    match provider {
        "openrouter" => vec![
            ModelInfo { id: "openai/gpt-4o".to_string(), name: "GPT-4o (via OpenRouter)".to_string(), description: Some("OpenAI's latest".to_string()) },
            ModelInfo { id: "anthropic/claude-3.5-sonnet".to_string(), name: "Claude 3.5 Sonnet".to_string(), description: Some("Anthropic's best".to_string()) },
            ModelInfo { id: "google/gemini-pro-1.5".to_string(), name: "Gemini Pro 1.5".to_string(), description: Some("Google's latest".to_string()) },
            ModelInfo { id: "meta-llama/llama-3.1-70b-instruct".to_string(), name: "Llama 3.1 70B".to_string(), description: Some("Open source".to_string()) },
        ],
        "mistral" => vec![
            ModelInfo { id: "mistral-large-latest".to_string(), name: "Mistral Large".to_string(), description: Some("Most capable".to_string()) },
            ModelInfo { id: "mistral-medium-latest".to_string(), name: "Mistral Medium".to_string(), description: Some("Balanced".to_string()) },
            ModelInfo { id: "mistral-small-latest".to_string(), name: "Mistral Small".to_string(), description: Some("Fast".to_string()) },
            ModelInfo { id: "codestral-latest".to_string(), name: "Codestral".to_string(), description: Some("Code specialist".to_string()) },
            ModelInfo { id: "open-mistral-nemo".to_string(), name: "Mistral Nemo".to_string(), description: Some("Open weight".to_string()) },
        ],
        "groq" => vec![
            ModelInfo { id: "llama-3.3-70b-versatile".to_string(), name: "Llama 3.3 70B".to_string(), description: Some("Fast inference".to_string()) },
            ModelInfo { id: "llama-3.1-8b-instant".to_string(), name: "Llama 3.1 8B".to_string(), description: Some("Fastest".to_string()) },
            ModelInfo { id: "mixtral-8x7b-32768".to_string(), name: "Mixtral 8x7B".to_string(), description: Some("MoE model".to_string()) },
            ModelInfo { id: "gemma2-9b-it".to_string(), name: "Gemma 2 9B".to_string(), description: Some("Google open".to_string()) },
        ],
        "together" => vec![
            ModelInfo { id: "meta-llama/Meta-Llama-3.1-405B-Instruct-Turbo".to_string(), name: "Llama 3.1 405B".to_string(), description: Some("Largest open".to_string()) },
            ModelInfo { id: "meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo".to_string(), name: "Llama 3.1 70B".to_string(), description: Some("Fast".to_string()) },
            ModelInfo { id: "mistralai/Mixtral-8x22B-Instruct-v0.1".to_string(), name: "Mixtral 8x22B".to_string(), description: Some("Large MoE".to_string()) },
            ModelInfo { id: "Qwen/Qwen2.5-72B-Instruct-Turbo".to_string(), name: "Qwen 2.5 72B".to_string(), description: Some("Alibaba".to_string()) },
        ],
        "cohere" => vec![
            ModelInfo { id: "command-r-plus".to_string(), name: "Command R+".to_string(), description: Some("Most capable".to_string()) },
            ModelInfo { id: "command-r".to_string(), name: "Command R".to_string(), description: Some("Balanced".to_string()) },
            ModelInfo { id: "command-light".to_string(), name: "Command Light".to_string(), description: Some("Fast".to_string()) },
        ],
        "deepseek" => vec![
            ModelInfo { id: "deepseek-chat".to_string(), name: "DeepSeek Chat".to_string(), description: Some("General purpose".to_string()) },
            ModelInfo { id: "deepseek-coder".to_string(), name: "DeepSeek Coder".to_string(), description: Some("Code specialist".to_string()) },
            ModelInfo { id: "deepseek-reasoner".to_string(), name: "DeepSeek Reasoner".to_string(), description: Some("Reasoning".to_string()) },
        ],
        _ => get_fallback_models(provider),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = LLMConfig::default();
        match config {
            LLMConfig::Ollama { host, model, .. } => {
                assert_eq!(host, "http://localhost:11434");
                assert_eq!(model, "llama3.2");
            }
            _ => panic!("Expected Ollama config"),
        }
    }

    #[test]
    fn test_message_builders() {
        let msg = ChatMessage::user("Hello");
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_request_builder() {
        let request = ChatRequest::new(vec![ChatMessage::user("Hi")])
            .with_system("You are a helpful assistant")
            .with_temperature(0.5)
            .with_max_tokens(1000);

        assert_eq!(request.system_prompt, Some("You are a helpful assistant".to_string()));
        assert_eq!(request.temperature, Some(0.5));
        assert_eq!(request.max_tokens, Some(1000));
    }
}
