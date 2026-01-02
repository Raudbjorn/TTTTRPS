//! OpenAI Provider Implementation
//!
//! This implementation is also used as a base for OpenAI-compatible providers.

use crate::core::llm::cost::{ProviderPricing, TokenUsage};
use crate::core::llm::router::{
    ChatChunk, ChatRequest, ChatResponse, LLMError, LLMProvider, MessageRole, Result,
};
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use std::time::Duration;
use tokio::sync::mpsc;

/// OpenAI provider
pub struct OpenAIProvider {
    api_key: String,
    model: String,
    max_tokens: u32,
    organization_id: Option<String>,
    base_url: String,
    client: Client,
}

impl OpenAIProvider {
    pub fn new(
        api_key: String,
        model: String,
        max_tokens: u32,
        organization_id: Option<String>,
        base_url: Option<String>,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            api_key,
            model,
            max_tokens,
            organization_id,
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
            client,
        }
    }

    pub fn gpt4o(api_key: String) -> Self {
        Self::new(api_key, "gpt-4o".to_string(), 4096, None, None)
    }

    pub fn gpt4o_mini(api_key: String) -> Self {
        Self::new(api_key, "gpt-4o-mini".to_string(), 4096, None, None)
    }

    fn build_messages(&self, request: &ChatRequest) -> Vec<serde_json::Value> {
        let mut messages = Vec::new();

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

        messages
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    fn id(&self) -> &str {
        "openai"
    }

    fn name(&self) -> &str {
        "OpenAI"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn health_check(&self) -> bool {
        let url = format!("{}/models", self.base_url);
        match self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
        {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    fn pricing(&self) -> Option<ProviderPricing> {
        ProviderPricing::for_model("openai", &self.model)
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/chat/completions", self.base_url);
        let messages = self.build_messages(&request);

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(self.max_tokens)
        });

        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        let start = std::time::Instant::now();
        let mut req_builder = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json");

        if let Some(org_id) = &self.organization_id {
            req_builder = req_builder.header("OpenAI-Organization", org_id);
        }

        let resp = req_builder.json(&body).send().await?;
        let status = resp.status();
        let latency = start.elapsed().as_millis() as u64;

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok())
                .unwrap_or(60);
            return Err(LLMError::RateLimited {
                retry_after_secs: retry_after,
            });
        }

        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(LLMError::AuthError("Invalid API key".to_string()));
        }

        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(LLMError::ApiError {
                status: status.as_u16(),
                message: text,
            });
        }

        let json: serde_json::Value = resp.json().await?;

        let content = json["choices"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|c| c["message"]["content"].as_str())
            .ok_or_else(|| LLMError::InvalidResponse("Missing content".to_string()))?
            .to_string();

        let usage = json["usage"].as_object().map(|u| TokenUsage {
            input_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            output_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
        });

        let cost = usage.as_ref().and_then(|u| {
            self.pricing().map(|p| p.calculate_cost(u))
        });

        let finish_reason = json["choices"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|c| c["finish_reason"].as_str())
            .map(|s| s.to_string());

        Ok(ChatResponse {
            content,
            model: json["model"].as_str().unwrap_or(&self.model).to_string(),
            provider: "openai".to_string(),
            usage,
            finish_reason,
            latency_ms: latency,
            cost_usd: cost,
        })
    }

    async fn stream_chat(
        &self,
        request: ChatRequest,
    ) -> Result<mpsc::Receiver<Result<ChatChunk>>> {
        let url = format!("{}/chat/completions", self.base_url);
        let messages = self.build_messages(&request);
        let stream_id = uuid::Uuid::new_v4().to_string();
        let model = self.model.clone();

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(self.max_tokens),
            "stream": true
        });

        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        let mut req_builder = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json");

        if let Some(org_id) = &self.organization_id {
            req_builder = req_builder.header("OpenAI-Organization", org_id);
        }

        let response = req_builder.json(&body).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(LLMError::ApiError { status, message: text });
        }

        let (tx, rx) = mpsc::channel(100);

        tokio::spawn(async move {
            let mut stream = response.bytes_stream();
            let mut chunk_index = 0u32;
            let mut final_usage: Option<TokenUsage> = None;

            while let Some(item) = stream.next().await {
                match item {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);

                        for line in text.lines() {
                            if line.starts_with("data: ") {
                                let data = &line[6..];
                                if data == "[DONE]" {
                                    let final_chunk = ChatChunk {
                                        stream_id: stream_id.clone(),
                                        content: String::new(),
                                        provider: "openai".to_string(),
                                        model: model.clone(),
                                        is_final: true,
                                        finish_reason: Some("stop".to_string()),
                                        usage: final_usage.clone(),
                                        index: chunk_index + 1,
                                    };
                                    let _ = tx.send(Ok(final_chunk)).await;
                                    return;
                                }

                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                    if let Some(delta) =
                                        json["choices"][0]["delta"]["content"].as_str()
                                    {
                                        if !delta.is_empty() {
                                            chunk_index += 1;
                                            let chunk = ChatChunk {
                                                stream_id: stream_id.clone(),
                                                content: delta.to_string(),
                                                provider: "openai".to_string(),
                                                model: model.clone(),
                                                is_final: false,
                                                finish_reason: None,
                                                usage: None,
                                                index: chunk_index,
                                            };
                                            if tx.send(Ok(chunk)).await.is_err() {
                                                return;
                                            }
                                        }
                                    }

                                    if let Some(reason) =
                                        json["choices"][0]["finish_reason"].as_str()
                                    {
                                        if reason != "null" {
                                            if let Some(usage) = json["usage"].as_object() {
                                                final_usage = Some(TokenUsage {
                                                    input_tokens: usage["prompt_tokens"]
                                                        .as_u64()
                                                        .unwrap_or(0)
                                                        as u32,
                                                    output_tokens: usage["completion_tokens"]
                                                        .as_u64()
                                                        .unwrap_or(0)
                                                        as u32,
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(LLMError::HttpError(e))).await;
                        return;
                    }
                }
            }
        });

        Ok(rx)
    }

    fn supports_embeddings(&self) -> bool {
        true
    }
}

/// Base implementation for OpenAI-compatible providers
/// This can be used by other providers (Groq, Together, etc.)
pub struct OpenAICompatibleProvider {
    id: String,
    name: String,
    api_key: String,
    model: String,
    max_tokens: u32,
    base_url: String,
    client: Client,
}

impl OpenAICompatibleProvider {
    pub fn new(
        id: String,
        name: String,
        api_key: String,
        model: String,
        max_tokens: u32,
        base_url: String,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            id,
            name,
            api_key,
            model,
            max_tokens,
            base_url,
            client,
        }
    }

    fn build_messages(&self, request: &ChatRequest) -> Vec<serde_json::Value> {
        let mut messages = Vec::new();

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

        messages
    }
}

#[async_trait]
impl LLMProvider for OpenAICompatibleProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn health_check(&self) -> bool {
        let url = format!("{}/models", self.base_url);
        match self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
        {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    fn pricing(&self) -> Option<ProviderPricing> {
        ProviderPricing::for_model(&self.id, &self.model)
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/chat/completions", self.base_url);
        let messages = self.build_messages(&request);

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(self.max_tokens)
        });

        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        let start = std::time::Instant::now();
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let latency = start.elapsed().as_millis() as u64;

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok())
                .unwrap_or(60);
            return Err(LLMError::RateLimited {
                retry_after_secs: retry_after,
            });
        }

        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(LLMError::ApiError {
                status: status.as_u16(),
                message: text,
            });
        }

        let json: serde_json::Value = resp.json().await?;

        let content = json["choices"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|c| c["message"]["content"].as_str())
            .ok_or_else(|| LLMError::InvalidResponse("Missing content".to_string()))?
            .to_string();

        let usage = json["usage"].as_object().map(|u| TokenUsage {
            input_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            output_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
        });

        let cost = usage.as_ref().and_then(|u| {
            self.pricing().map(|p| p.calculate_cost(u))
        });

        Ok(ChatResponse {
            content,
            model: json["model"].as_str().unwrap_or(&self.model).to_string(),
            provider: self.id.clone(),
            usage,
            finish_reason: json["choices"]
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|c| c["finish_reason"].as_str())
                .map(|s| s.to_string()),
            latency_ms: latency,
            cost_usd: cost,
        })
    }

    async fn stream_chat(
        &self,
        request: ChatRequest,
    ) -> Result<mpsc::Receiver<Result<ChatChunk>>> {
        let url = format!("{}/chat/completions", self.base_url);
        let messages = self.build_messages(&request);
        let stream_id = uuid::Uuid::new_v4().to_string();
        let model = self.model.clone();
        let provider_id = self.id.clone();

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(self.max_tokens),
            "stream": true
        });

        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(LLMError::ApiError { status, message: text });
        }

        let (tx, rx) = mpsc::channel(100);

        tokio::spawn(async move {
            let mut stream = response.bytes_stream();
            let mut chunk_index = 0u32;
            let mut final_usage: Option<TokenUsage> = None;

            while let Some(item) = stream.next().await {
                match item {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);

                        for line in text.lines() {
                            if line.starts_with("data: ") {
                                let data = &line[6..];
                                if data == "[DONE]" {
                                    let final_chunk = ChatChunk {
                                        stream_id: stream_id.clone(),
                                        content: String::new(),
                                        provider: provider_id.clone(),
                                        model: model.clone(),
                                        is_final: true,
                                        finish_reason: Some("stop".to_string()),
                                        usage: final_usage.clone(),
                                        index: chunk_index + 1,
                                    };
                                    let _ = tx.send(Ok(final_chunk)).await;
                                    return;
                                }

                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                    if let Some(delta) =
                                        json["choices"][0]["delta"]["content"].as_str()
                                    {
                                        if !delta.is_empty() {
                                            chunk_index += 1;
                                            let chunk = ChatChunk {
                                                stream_id: stream_id.clone(),
                                                content: delta.to_string(),
                                                provider: provider_id.clone(),
                                                model: model.clone(),
                                                is_final: false,
                                                finish_reason: None,
                                                usage: None,
                                                index: chunk_index,
                                            };
                                            if tx.send(Ok(chunk)).await.is_err() {
                                                return;
                                            }
                                        }
                                    }

                                    if let Some(usage) = json["usage"].as_object() {
                                        final_usage = Some(TokenUsage {
                                            input_tokens: usage["prompt_tokens"]
                                                .as_u64()
                                                .unwrap_or(0)
                                                as u32,
                                            output_tokens: usage["completion_tokens"]
                                                .as_u64()
                                                .unwrap_or(0)
                                                as u32,
                                        });
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(LLMError::HttpError(e))).await;
                        return;
                    }
                }
            }
        });

        Ok(rx)
    }
}
