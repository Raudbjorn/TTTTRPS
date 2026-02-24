//! Meilisearch Provider Implementation
//!
//! RAG-powered provider using Meilisearch's Chat API.

use crate::core::llm::cost::ProviderPricing;
use crate::core::llm::router::{
    ChatChunk, ChatRequest, ChatResponse, LLMError, LLMProvider, MessageRole, Result,
};
use crate::core::search_chat::{MeilisearchChatClient, ChatMessage as MeiliMessage, ChatCompletionRequest};
use async_trait::async_trait;
use tokio::sync::mpsc;

/// Meilisearch provider for RAG chat
pub struct MeilisearchProvider {
    client: MeilisearchChatClient,
    workspace_id: String,
    model: String,
}

impl MeilisearchProvider {
    pub fn new(host: String, api_key: Option<String>, workspace_id: String, model: String) -> Self {
        Self {
            client: MeilisearchChatClient::new(&host, api_key.as_deref()),
            workspace_id,
            model,
        }
    }

    fn build_messages(&self, request: &ChatRequest) -> Vec<MeiliMessage> {
        let mut messages = Vec::new();

        if let Some(system) = &request.system_prompt {
           messages.push(MeiliMessage::system(system));
        }

        for msg in &request.messages {
            match msg.role {
                MessageRole::System => messages.push(MeiliMessage::system(&msg.content)),
                MessageRole::User => messages.push(MeiliMessage::user(&msg.content)),
                MessageRole::Assistant => messages.push(MeiliMessage::assistant(&msg.content)),
            }
        }

        messages
    }
}

#[async_trait]
impl LLMProvider for MeilisearchProvider {
    fn id(&self) -> &str {
        "meilisearch"
    }

    fn name(&self) -> &str {
        "Meilisearch RAG"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn health_check(&self) -> bool {
        // Simple check by trying to get settings
        self.client.get_workspace_settings(&self.workspace_id).await.is_ok()
    }

    fn pricing(&self) -> Option<ProviderPricing> {
        // Meilisearch itself doesn't have per-token pricing, depends on backend
        None
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let messages = self.build_messages(&request);
        let start = std::time::Instant::now();

        let content = self.client.chat_completion(
            &self.workspace_id,
            messages,
            &self.model
        ).await.map_err(|e| LLMError::ApiError { status: 500, message: e })?;

        let latency = start.elapsed().as_millis() as u64;

        Ok(ChatResponse {
            content,
            model: self.model.clone(),
            provider: "meilisearch".to_string(),
            usage: None, // Meilisearch response doesn't currently return usage
            finish_reason: Some("stop".to_string()),
            latency_ms: latency,
            cost_usd: None,
            tool_calls: None,
        })
    }

    async fn stream_chat(
        &self,
        request: ChatRequest,
    ) -> Result<mpsc::Receiver<Result<ChatChunk>>> {
        let messages = self.build_messages(&request);
        let stream_id = uuid::Uuid::new_v4().to_string();
        let model = self.model.clone();

        let meili_request = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            stream: true,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            tools: Some(vec![
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": "_meiliSearchProgress",
                        "description": "Reports real-time search progress to the user"
                    }
                }),
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": "_meiliSearchSources",
                        "description": "Provides sources and references for the information"
                    }
                })
            ]),
        };

        let mut rx = self.client.chat_completion_stream(&self.workspace_id, meili_request)
            .await
            .map_err(|e| LLMError::ApiError { status: 500, message: e })?;

        let (tx, proxy_rx) = mpsc::channel(100);

        tokio::spawn(async move {
            let mut chunk_index = 0;

            while let Some(result) = rx.recv().await {
                match result {
                    Ok(content) => {
                        if content == "[DONE]" {
                            let final_chunk = ChatChunk {
                                stream_id: stream_id.clone(),
                                content: String::new(),
                                provider: "meilisearch".to_string(),
                                model: model.clone(),
                                is_final: true,
                                finish_reason: Some("stop".to_string()),
                                usage: None,
                                index: chunk_index + 1,
                            };
                            let _ = tx.send(Ok(final_chunk)).await;
                            return;
                        }

                        chunk_index += 1;
                        let chunk = ChatChunk {
                            stream_id: stream_id.clone(),
                            content,
                            provider: "meilisearch".to_string(),
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
                    Err(e) => {
                        let _ = tx.send(Err(LLMError::ApiError { status: 500, message: e })).await;
                        return;
                    }
                }
            }

            // Stream ended without [DONE] - send a final chunk to prevent consumers from waiting forever
            let final_chunk = ChatChunk {
                stream_id: stream_id.clone(),
                content: String::new(),
                provider: "meilisearch".to_string(),
                model: model.clone(),
                is_final: true,
                finish_reason: Some("stream_terminated".to_string()),
                usage: None,
                index: chunk_index + 1,
            };
            let _ = tx.send(Ok(final_chunk)).await;
        });

        Ok(proxy_rx)
    }

    fn supports_streaming(&self) -> bool {
        true
    }
}
