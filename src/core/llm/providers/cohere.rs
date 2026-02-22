//! Cohere Provider Implementation
//!
//! Cohere provides Command models with strong RAG and enterprise capabilities.

use crate::core::llm::cost::{ProviderPricing, TokenUsage};
use crate::core::llm::router::{
    ChatChunk, ChatRequest, ChatResponse, LLMError, LLMProvider, MessageRole, Result,
};
use async_trait::async_trait;
use reqwest::Client;
use std::time::Duration;
use tokio::sync::mpsc;

const COHERE_CHAT_URL: &str = "https://api.cohere.ai/v1/chat";

/// Cohere provider
pub struct CohereProvider {
    api_key: String,
    model: String,
    client: Client,
}

impl CohereProvider {
    pub fn new(api_key: String, model: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            api_key,
            model,
            client,
        }
    }

    /// Use Command R+ (most capable)
    pub fn command_r_plus(api_key: String) -> Self {
        Self::new(api_key, "command-r-plus".to_string())
    }

    /// Use Command R
    pub fn command_r(api_key: String) -> Self {
        Self::new(api_key, "command-r".to_string())
    }

    /// Use Command Light (fastest)
    pub fn command_light(api_key: String) -> Self {
        Self::new(api_key, "command-light".to_string())
    }
}

#[async_trait]
impl LLMProvider for CohereProvider {
    fn id(&self) -> &str {
        "cohere"
    }

    fn name(&self) -> &str {
        "Cohere"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn health_check(&self) -> bool {
        !self.api_key.is_empty()
    }

    fn pricing(&self) -> Option<ProviderPricing> {
        ProviderPricing::for_model("cohere", &self.model)
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        // Build chat history (all messages except the last one)
        let mut chat_history: Vec<serde_json::Value> = Vec::new();

        for msg in request
            .messages
            .iter()
            .take(request.messages.len().saturating_sub(1))
        {
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
        let message = request
            .messages
            .last()
            .map(|m| m.content.clone())
            .unwrap_or_default();

        let mut body = serde_json::json!({
            "model": self.model,
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

        let start = std::time::Instant::now();
        let resp = self
            .client
            .post(COHERE_CHAT_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let latency = start.elapsed().as_millis() as u64;

        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(LLMError::ApiError {
                status: status.as_u16(),
                message: text,
            });
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

        let cost = usage.as_ref().and_then(|u| {
            self.pricing().map(|p| p.calculate_cost(u))
        });

        Ok(ChatResponse {
            content,
            model: self.model.clone(),
            provider: "cohere".to_string(),
            usage,
            finish_reason: json["finish_reason"].as_str().map(|s| s.to_string()),
            latency_ms: latency,
            cost_usd: cost,
            tool_calls: None,
        })
    }

    async fn stream_chat(
        &self,
        request: ChatRequest,
    ) -> Result<mpsc::Receiver<Result<ChatChunk>>> {
        // Cohere streaming uses a different format - for now, we simulate with non-streaming
        // and return results as a single chunk. A full implementation would use their
        // streaming endpoint with stream: true.

        // For a production implementation, you would implement proper SSE streaming
        // similar to the other providers.

        let (tx, rx) = mpsc::channel(100);
        let stream_id = uuid::Uuid::new_v4().to_string();
        let model = self.model.clone();

        // Clone self for the async task
        let api_key = self.api_key.clone();
        let model_clone = self.model.clone();
        let client = self.client.clone();

        // Build the request body
        let mut chat_history: Vec<serde_json::Value> = Vec::new();
        for msg in request
            .messages
            .iter()
            .take(request.messages.len().saturating_sub(1))
        {
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

        let message = request
            .messages
            .last()
            .map(|m| m.content.clone())
            .unwrap_or_default();

        let mut body = serde_json::json!({
            "model": model_clone,
            "message": message,
            "stream": true
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

        tokio::spawn(async move {
            let response = client
                .post(COHERE_CHAT_URL)
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        let status = resp.status().as_u16();
                        let text = resp.text().await.unwrap_or_default();
                        let _ = tx
                            .send(Err(LLMError::ApiError {
                                status,
                                message: text,
                            }))
                            .await;
                        return;
                    }

                    use futures_util::StreamExt;
                    let mut stream = resp.bytes_stream();
                    let mut chunk_index = 0u32;
                    let mut final_usage: Option<TokenUsage> = None;

                    while let Some(item) = stream.next().await {
                        match item {
                            Ok(bytes) => {
                                let text = String::from_utf8_lossy(&bytes);

                                for line in text.lines() {
                                    if line.is_empty() {
                                        continue;
                                    }

                                    if let Ok(json) =
                                        serde_json::from_str::<serde_json::Value>(line)
                                    {
                                        let event_type =
                                            json["event_type"].as_str().unwrap_or("");

                                        match event_type {
                                            "text-generation" => {
                                                if let Some(text) = json["text"].as_str() {
                                                    if !text.is_empty() {
                                                        chunk_index += 1;
                                                        let chunk = ChatChunk {
                                                            stream_id: stream_id.clone(),
                                                            content: text.to_string(),
                                                            provider: "cohere".to_string(),
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
                                            "stream-end" => {
                                                if let Some(response) =
                                                    json["response"].as_object()
                                                {
                                                    if let Some(meta) = response
                                                        .get("meta")
                                                        .and_then(|m| m.get("tokens"))
                                                        .and_then(|t| t.as_object())
                                                    {
                                                        final_usage = Some(TokenUsage {
                                                            input_tokens: meta["input_tokens"]
                                                                .as_u64()
                                                                .unwrap_or(0)
                                                                as u32,
                                                            output_tokens: meta["output_tokens"]
                                                                .as_u64()
                                                                .unwrap_or(0)
                                                                as u32,
                                                        });
                                                    }
                                                }

                                                let final_chunk = ChatChunk {
                                                    stream_id: stream_id.clone(),
                                                    content: String::new(),
                                                    provider: "cohere".to_string(),
                                                    model: model.clone(),
                                                    is_final: true,
                                                    finish_reason: json["finish_reason"]
                                                        .as_str()
                                                        .map(|s| s.to_string()),
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
                            Err(e) => {
                                let _ = tx.send(Err(LLMError::HttpError(e))).await;
                                return;
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(LLMError::HttpError(e))).await;
                }
            }
        });

        Ok(rx)
    }

    fn supports_embeddings(&self) -> bool {
        true
    }
}
