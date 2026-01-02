//! Google Gemini Provider Implementation

use crate::core::llm::cost::{ProviderPricing, TokenUsage};
use crate::core::llm::router::{
    ChatChunk, ChatMessage, ChatRequest, ChatResponse, LLMError, LLMProvider, MessageRole, Result,
};
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use std::time::Duration;
use tokio::sync::mpsc;

/// Google Gemini provider
pub struct GeminiProvider {
    api_key: String,
    model: String,
    client: Client,
}

impl GeminiProvider {
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

    pub fn flash(api_key: String) -> Self {
        Self::new(api_key, "gemini-2.0-flash-exp".to_string())
    }

    pub fn pro(api_key: String) -> Self {
        Self::new(api_key, "gemini-1.5-pro".to_string())
    }

    fn build_contents(&self, request: &ChatRequest) -> Vec<serde_json::Value> {
        request
            .messages
            .iter()
            .filter_map(|msg| {
                let role = match msg.role {
                    MessageRole::User => "user",
                    MessageRole::Assistant => "model",
                    MessageRole::System => return None,
                };
                Some(serde_json::json!({
                    "role": role,
                    "parts": [{ "text": msg.content }]
                }))
            })
            .collect()
    }
}

#[async_trait]
impl LLMProvider for GeminiProvider {
    fn id(&self) -> &str {
        "gemini"
    }

    fn name(&self) -> &str {
        "Google Gemini"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn health_check(&self) -> bool {
        self.api_key.starts_with("AIza")
    }

    fn pricing(&self) -> Option<ProviderPricing> {
        ProviderPricing::for_model("gemini", &self.model)
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let contents = self.build_contents(&request);

        let mut body = serde_json::json!({ "contents": contents });

        if let Some(system) = &request.system_prompt {
            body["systemInstruction"] = serde_json::json!({
                "parts": [{ "text": system }]
            });
        }

        if request.temperature.is_some() || request.max_tokens.is_some() {
            let mut gen_config = serde_json::Map::new();
            if let Some(temp) = request.temperature {
                gen_config.insert("temperature".to_string(), serde_json::json!(temp));
            }
            if let Some(max) = request.max_tokens {
                gen_config.insert("maxOutputTokens".to_string(), serde_json::json!(max));
            }
            body["generationConfig"] = serde_json::Value::Object(gen_config);
        }

        let start = std::time::Instant::now();
        let resp = self
            .client
            .post(&url)
            .header("content-type", "application/json")
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

        let cost = usage.as_ref().and_then(|u| {
            self.pricing().map(|p| p.calculate_cost(u))
        });

        Ok(ChatResponse {
            content,
            model: self.model.clone(),
            provider: "gemini".to_string(),
            usage,
            finish_reason: json["candidates"]
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|c| c["finishReason"].as_str())
                .map(|s| s.to_string()),
            latency_ms: latency,
            cost_usd: cost,
        })
    }

    async fn stream_chat(
        &self,
        request: ChatRequest,
    ) -> Result<mpsc::Receiver<Result<ChatChunk>>> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?key={}&alt=sse",
            self.model, self.api_key
        );

        let contents = self.build_contents(&request);
        let stream_id = uuid::Uuid::new_v4().to_string();
        let model = self.model.clone();

        let mut body = serde_json::json!({ "contents": contents });

        if let Some(system) = &request.system_prompt {
            body["systemInstruction"] = serde_json::json!({
                "parts": [{ "text": system }]
            });
        }

        if request.temperature.is_some() || request.max_tokens.is_some() {
            let mut gen_config = serde_json::Map::new();
            if let Some(temp) = request.temperature {
                gen_config.insert("temperature".to_string(), serde_json::json!(temp));
            }
            if let Some(max) = request.max_tokens {
                gen_config.insert("maxOutputTokens".to_string(), serde_json::json!(max));
            }
            body["generationConfig"] = serde_json::Value::Object(gen_config);
        }

        let response = self
            .client
            .post(&url)
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
            let mut final_usage: Option<TokenUsage> = None;

            while let Some(item) = stream.next().await {
                match item {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);

                        for line in text.lines() {
                            if line.starts_with("data: ") {
                                let data = &line[6..];
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                    if let Some(text) =
                                        json["candidates"][0]["content"]["parts"][0]["text"]
                                            .as_str()
                                    {
                                        if !text.is_empty() {
                                            chunk_index += 1;
                                            let chunk = ChatChunk {
                                                stream_id: stream_id.clone(),
                                                content: text.to_string(),
                                                provider: "gemini".to_string(),
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

                                    if let Some(usage) = json["usageMetadata"].as_object() {
                                        final_usage = Some(TokenUsage {
                                            input_tokens: usage["promptTokenCount"]
                                                .as_u64()
                                                .unwrap_or(0)
                                                as u32,
                                            output_tokens: usage["candidatesTokenCount"]
                                                .as_u64()
                                                .unwrap_or(0)
                                                as u32,
                                        });
                                    }

                                    if let Some(reason) =
                                        json["candidates"][0]["finishReason"].as_str()
                                    {
                                        if reason == "STOP" {
                                            let final_chunk = ChatChunk {
                                                stream_id: stream_id.clone(),
                                                content: String::new(),
                                                provider: "gemini".to_string(),
                                                model: model.clone(),
                                                is_final: true,
                                                finish_reason: Some(reason.to_string()),
                                                usage: final_usage.clone(),
                                                index: chunk_index + 1,
                                            };
                                            let _ = tx.send(Ok(final_chunk)).await;
                                            return;
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

            // Send final chunk if stream ended without explicit STOP
            let final_chunk = ChatChunk {
                stream_id: stream_id.clone(),
                content: String::new(),
                provider: "gemini".to_string(),
                model: model.clone(),
                is_final: true,
                finish_reason: Some("stop".to_string()),
                usage: final_usage,
                index: chunk_index + 1,
            };
            let _ = tx.send(Ok(final_chunk)).await;
        });

        Ok(rx)
    }

    fn supports_embeddings(&self) -> bool {
        true
    }
}
