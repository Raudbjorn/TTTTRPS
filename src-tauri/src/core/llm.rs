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
                // Claude doesn't have native embeddings, would need Voyage API
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
                // Simple validation - just check if key looks valid
                Ok(api_key.starts_with("sk-ant-"))
            }
            LLMConfig::Gemini { api_key, .. } => {
                // Simple validation
                Ok(api_key.starts_with("AIza"))
            }
            LLMConfig::OpenAI { api_key, base_url, .. } => {
                // Check models endpoint to validate API key
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
