//! Ollama Provider Implementation
//!
//! Local LLM provider using Ollama for running models locally.

use crate::core::llm::cost::{ProviderPricing, TokenUsage};
use crate::core::llm::router::{
    ChatChunk, ChatRequest, ChatResponse, LLMError, LLMProvider, MessageRole, Result,
};
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use std::time::Duration;
use tokio::sync::mpsc;

/// Ollama provider for local LLM inference
pub struct OllamaProvider {
    host: String,
    model: String,
    client: Client,
}

impl OllamaProvider {
    /// Create a new Ollama provider
    pub fn new(host: String, model: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("Failed to create HTTP client");

        Self { host, model, client }
    }

    /// Create with default localhost
    pub fn localhost(model: String) -> Self {
        Self::new("http://localhost:11434".to_string(), model)
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
impl LLMProvider for OllamaProvider {
    fn id(&self) -> &str {
        "ollama"
    }

    fn name(&self) -> &str {
        "Ollama"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn health_check(&self) -> bool {
        let url = format!("{}/api/tags", self.host);
        match self.client.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    fn pricing(&self) -> Option<ProviderPricing> {
        Some(ProviderPricing::free("ollama", &self.model))
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/api/chat", self.host);
        let messages = self.build_messages(&request);

        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
            "options": {
                "temperature": request.temperature.unwrap_or(0.7)
            }
        });

        let start = std::time::Instant::now();
        let resp = self.client.post(&url).json(&body).send().await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(LLMError::ApiError { status, message: text });
        }

        let json: serde_json::Value = resp.json().await?;
        let latency = start.elapsed().as_millis() as u64;

        let content = json["message"]["content"]
            .as_str()
            .ok_or_else(|| LLMError::InvalidResponse("Missing content".to_string()))?
            .to_string();

        Ok(ChatResponse {
            content,
            model: self.model.clone(),
            provider: "ollama".to_string(),
            usage: Some(TokenUsage {
                input_tokens: json["prompt_eval_count"].as_u64().unwrap_or(0) as u32,
                output_tokens: json["eval_count"].as_u64().unwrap_or(0) as u32,
            }),
            finish_reason: Some("stop".to_string()),
            latency_ms: latency,
            cost_usd: Some(0.0),
            tool_calls: None,
        })
    }

    async fn stream_chat(
        &self,
        request: ChatRequest,
    ) -> Result<mpsc::Receiver<Result<ChatChunk>>> {
        let url = format!("{}/api/chat", self.host);
        let messages = self.build_messages(&request);
        let stream_id = uuid::Uuid::new_v4().to_string();
        let model = self.model.clone();

        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": true,
            "options": {
                "temperature": request.temperature.unwrap_or(0.7)
            }
        });

        let response = self.client.post(&url).json(&body).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(LLMError::ApiError { status, message: text });
        }

        let (tx, rx) = mpsc::channel(100);

        tokio::spawn(async move {
            let mut stream = response.bytes_stream();
            let mut chunk_index = 0u32;

            while let Some(item) = stream.next().await {
                match item {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);

                        for line in text.lines() {
                            if line.is_empty() {
                                continue;
                            }

                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                                if let Some(content) = json["message"]["content"].as_str() {
                                    if !content.is_empty() {
                                        chunk_index += 1;
                                        let chunk = ChatChunk {
                                            stream_id: stream_id.clone(),
                                            content: content.to_string(),
                                            provider: "ollama".to_string(),
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

                                if json["done"].as_bool().unwrap_or(false) {
                                    let input_tokens =
                                        json["prompt_eval_count"].as_u64().unwrap_or(0) as u32;
                                    let output_tokens =
                                        json["eval_count"].as_u64().unwrap_or(0) as u32;

                                    let final_chunk = ChatChunk {
                                        stream_id: stream_id.clone(),
                                        content: String::new(),
                                        provider: "ollama".to_string(),
                                        model: model.clone(),
                                        is_final: true,
                                        finish_reason: Some("stop".to_string()),
                                        usage: Some(TokenUsage {
                                            input_tokens,
                                            output_tokens,
                                        }),
                                        index: chunk_index + 1,
                                    };
                                    let _ = tx.send(Ok(final_chunk)).await;
                                    return;
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
