//! Claude/Anthropic Provider Implementation

use crate::core::llm::cost::{ProviderPricing, TokenUsage};
use crate::core::llm::router::{
    ChatChunk, ChatMessage, ChatRequest, ChatResponse, LLMError, LLMProvider, MessageRole, Result,
};
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use std::time::Duration;
use tokio::sync::mpsc;

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";

/// Claude/Anthropic provider
pub struct ClaudeProvider {
    api_key: String,
    model: String,
    max_tokens: u32,
    client: Client,
}

impl ClaudeProvider {
    pub fn new(api_key: String, model: String, max_tokens: u32) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            api_key,
            model,
            max_tokens,
            client,
        }
    }

    pub fn sonnet(api_key: String) -> Self {
        Self::new(api_key, "claude-sonnet-4-20250514".to_string(), 8192)
    }

    fn build_request(&self, request: &ChatRequest) -> serde_json::Value {
        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .filter(|m| m.role != MessageRole::System)
            .map(|m| {
                serde_json::json!({
                    "role": match m.role {
                        MessageRole::User => "user",
                        MessageRole::Assistant => "assistant",
                        MessageRole::System => "user",
                    },
                    "content": m.content
                })
            })
            .collect();

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(self.max_tokens)
        });

        if let Some(system) = &request.system_prompt {
            body["system"] = serde_json::Value::String(system.clone());
        }

        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        body
    }
}

#[async_trait]
impl LLMProvider for ClaudeProvider {
    fn id(&self) -> &str {
        "claude"
    }

    fn name(&self) -> &str {
        "Claude"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn health_check(&self) -> bool {
        self.api_key.starts_with("sk-ant-")
    }

    fn pricing(&self) -> Option<ProviderPricing> {
        ProviderPricing::for_model("claude", &self.model)
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let body = self.build_request(&request);

        let start = std::time::Instant::now();
        let resp = self
            .client
            .post(API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
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

        let cost = usage.as_ref().and_then(|u| {
            self.pricing().map(|p| p.calculate_cost(u))
        });

        Ok(ChatResponse {
            content,
            model: json["model"].as_str().unwrap_or(&self.model).to_string(),
            provider: "claude".to_string(),
            usage,
            finish_reason: json["stop_reason"].as_str().map(|s| s.to_string()),
            latency_ms: latency,
            cost_usd: cost,
        })
    }

    async fn stream_chat(
        &self,
        request: ChatRequest,
    ) -> Result<mpsc::Receiver<Result<ChatChunk>>> {
        let mut body = self.build_request(&request);
        body["stream"] = serde_json::json!(true);

        let stream_id = uuid::Uuid::new_v4().to_string();
        let model = self.model.clone();

        let response = self
            .client
            .post(API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
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
            let mut input_tokens = 0u32;
            let mut final_usage: Option<TokenUsage> = None;

            while let Some(item) = stream.next().await {
                match item {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);

                        for line in text.lines() {
                            if line.starts_with("data: ") {
                                let data = &line[6..];
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                    let event_type = json["type"].as_str().unwrap_or("");

                                    match event_type {
                                        "message_start" => {
                                            if let Some(usage) =
                                                json["message"]["usage"].as_object()
                                            {
                                                input_tokens = usage["input_tokens"]
                                                    .as_u64()
                                                    .unwrap_or(0)
                                                    as u32;
                                            }
                                        }
                                        "content_block_delta" => {
                                            if let Some(delta) = json["delta"]["text"].as_str() {
                                                if !delta.is_empty() {
                                                    chunk_index += 1;
                                                    let chunk = ChatChunk {
                                                        stream_id: stream_id.clone(),
                                                        content: delta.to_string(),
                                                        provider: "claude".to_string(),
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
                                        }
                                        "message_delta" => {
                                            if let Some(usage) = json["usage"].as_object() {
                                                let output_tokens = usage["output_tokens"]
                                                    .as_u64()
                                                    .unwrap_or(0)
                                                    as u32;
                                                final_usage = Some(TokenUsage {
                                                    input_tokens,
                                                    output_tokens,
                                                });
                                            }
                                        }
                                        "message_stop" => {
                                            let final_chunk = ChatChunk {
                                                stream_id: stream_id.clone(),
                                                content: String::new(),
                                                provider: "claude".to_string(),
                                                model: model.clone(),
                                                is_final: true,
                                                finish_reason: Some("stop".to_string()),
                                                usage: final_usage.clone(),
                                                index: chunk_index + 1,
                                            };
                                            let _ = tx.send(Ok(final_chunk)).await;
                                            return;
                                        }
                                        _ => {}
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
