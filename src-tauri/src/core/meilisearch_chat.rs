//! Meilisearch Chat Module
//!
//! Provides RAG-powered conversational search using Meilisearch's Chat API.
//! This enables the DM to have context-aware conversations that automatically
//! cite relevant documents (rules, lore, etc.) in responses.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use futures_util::StreamExt;
use tokio::sync::mpsc;

// ============================================================================
// Configuration Types
// ============================================================================

/// LLM provider source for Meilisearch Chat
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChatLLMSource {
    OpenAi,
    AzureOpenAi,
    Mistral,
    Gemini,
    VLlm,
}

impl Default for ChatLLMSource {
    fn default() -> Self {
        Self::OpenAi
    }
}

/// Prompt configuration for chat workspace
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatPrompts {
    /// System prompt that defines the AI's behavior
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Description of the search tool for the AI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_description: Option<String>,
    /// Description of the query parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_q_param: Option<String>,
    /// Description of the index selection parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_index_uid_param: Option<String>,
}

/// Workspace settings for Meilisearch Chat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatWorkspaceSettings {
    /// LLM provider source
    pub source: ChatLLMSource,
    /// API key for the LLM provider
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Model to use (e.g., "gpt-4o", "gpt-3.5-turbo")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Prompt configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<ChatPrompts>,
    /// Base URL for vLLM or custom endpoints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

impl Default for ChatWorkspaceSettings {
    fn default() -> Self {
        Self {
            source: ChatLLMSource::OpenAi,
            api_key: None,
            model: Some("gpt-4o-mini".to_string()),
            prompts: Some(ChatPrompts {
                system: Some(DEFAULT_DM_SYSTEM_PROMPT.to_string()),
                ..Default::default()
            }),
            base_url: None,
        }
    }
}

/// Default system prompt for the DM persona
pub const DEFAULT_DM_SYSTEM_PROMPT: &str = r#"You are an expert Dungeon Master assistant for tabletop role-playing games.

Your role is to:
- Help Game Masters run engaging sessions
- Provide rules clarifications citing specific sources
- Generate creative content (NPCs, locations, plot hooks)
- Answer questions about game mechanics
- Suggest narrative ideas that fit the campaign's tone

When answering questions:
- Search the available indexes for relevant rules and lore
- Cite your sources when providing rules information
- Be concise but thorough
- Maintain the tone appropriate to the game being played

You have access to the player's rulebooks, campaign notes, and lore documents.
Use the search tool to find relevant information before answering."#;

// ============================================================================
// Chat Message Types (OpenAI Compatible)
// ============================================================================

/// A chat message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: &str) -> Self {
        Self {
            role: "user".to_string(),
            content: content.to_string(),
        }
    }

    pub fn assistant(content: &str) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.to_string(),
        }
    }

    pub fn system(content: &str) -> Self {
        Self {
            role: "system".to_string(),
            content: content.to_string(),
        }
    }
}

/// Chat completion request
#[derive(Debug, Clone, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

/// Streaming response delta
#[derive(Debug, Clone, Deserialize)]
pub struct StreamDelta {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
}

/// Streaming response choice
#[derive(Debug, Clone, Deserialize)]
pub struct StreamChoice {
    pub delta: StreamDelta,
    #[serde(default)]
    pub index: u32,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

/// Streaming response chunk
#[derive(Debug, Clone, Deserialize)]
pub struct StreamChunk {
    pub id: String,
    pub choices: Vec<StreamChoice>,
    #[serde(default)]
    pub model: Option<String>,
}

// ============================================================================
// Chat Client
// ============================================================================

/// Meilisearch Chat Client
pub struct MeilisearchChatClient {
    host: String,
    api_key: Option<String>,
    http_client: reqwest::Client,
}

impl MeilisearchChatClient {
    pub fn new(host: &str, api_key: Option<&str>) -> Self {
        Self {
            host: host.to_string(),
            api_key: api_key.map(|s| s.to_string()),
            http_client: reqwest::Client::new(),
        }
    }

    /// Enable experimental chat features
    pub async fn enable_chat_feature(&self) -> Result<(), String> {
        let url = format!("{}/experimental-features", self.host);

        let mut request = self.http_client
            .patch(&url)
            .json(&serde_json::json!({
                "chat": true
            }));

        if let Some(key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response = request.send().await
            .map_err(|e| format!("Failed to enable chat feature: {}", e))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(format!("Failed to enable chat feature: {}", error));
        }

        log::info!("Meilisearch chat feature enabled");
        Ok(())
    }

    /// Configure a chat workspace
    pub async fn configure_workspace(
        &self,
        workspace_id: &str,
        settings: &ChatWorkspaceSettings,
    ) -> Result<(), String> {
        let url = format!("{}/chats/{}/settings", self.host, workspace_id);

        let mut request = self.http_client
            .patch(&url)
            .json(settings);

        if let Some(key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response = request.send().await
            .map_err(|e| format!("Failed to configure workspace: {}", e))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(format!("Failed to configure workspace: {}", error));
        }

        log::info!("Configured chat workspace: {}", workspace_id);
        Ok(())
    }

    /// Get workspace settings
    pub async fn get_workspace_settings(
        &self,
        workspace_id: &str,
    ) -> Result<Option<ChatWorkspaceSettings>, String> {
        let url = format!("{}/chats/{}/settings", self.host, workspace_id);

        let mut request = self.http_client.get(&url);

        if let Some(key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response = request.send().await
            .map_err(|e| format!("Failed to get workspace settings: {}", e))?;

        if response.status().is_success() {
            let settings = response.json().await
                .map_err(|e| format!("Failed to parse settings: {}", e))?;
            Ok(Some(settings))
        } else if response.status().as_u16() == 404 {
            Ok(None)
        } else {
            let error = response.text().await.unwrap_or_default();
            Err(format!("Failed to get workspace settings: {}", error))
        }
    }

    /// Create a chat completion with streaming
    pub async fn chat_completion_stream(
        &self,
        workspace_id: &str,
        request: ChatCompletionRequest,
    ) -> Result<mpsc::Receiver<Result<String, String>>, String> {
        let url = format!("{}/chats/{}/chat/completions", self.host, workspace_id);

        let mut http_request = self.http_client
            .post(&url)
            .json(&request);

        if let Some(key) = &self.api_key {
            http_request = http_request.header("Authorization", format!("Bearer {}", key));
        }

        let response = http_request.send().await
            .map_err(|e| format!("Chat request failed: {}", e))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(format!("Chat completion failed: {}", error));
        }

        // Create channel for streaming responses
        let (tx, rx) = mpsc::channel(100);

        // Spawn task to process SSE stream
        tokio::spawn(async move {
            let mut stream = response.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));

                        // Process complete SSE events
                        while let Some(pos) = buffer.find("\n\n") {
                            let event = buffer[..pos].to_string();
                            buffer = buffer[pos + 2..].to_string();

                            // Parse SSE event
                            for line in event.lines() {
                                if line.starts_with("data: ") {
                                    let data = &line[6..];
                                    if data == "[DONE]" {
                                        let _ = tx.send(Ok("[DONE]".to_string())).await;
                                        return;
                                    }

                                    // Parse JSON chunk
                                    if let Ok(chunk) = serde_json::from_str::<StreamChunk>(data) {
                                        for choice in chunk.choices {
                                            if let Some(content) = choice.delta.content {
                                                let _ = tx.send(Ok(content)).await;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e.to_string())).await;
                        return;
                    }
                }
            }
        });

        Ok(rx)
    }

    /// Non-streaming chat completion (collects full response)
    pub async fn chat_completion(
        &self,
        workspace_id: &str,
        messages: Vec<ChatMessage>,
        model: &str,
    ) -> Result<String, String> {
        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages,
            stream: true, // Meilisearch only supports streaming
            temperature: Some(0.7),
            max_tokens: Some(2048),
        };

        let mut rx = self.chat_completion_stream(workspace_id, request).await?;
        let mut full_response = String::new();

        while let Some(result) = rx.recv().await {
            match result {
                Ok(content) => {
                    if content == "[DONE]" {
                        break;
                    }
                    full_response.push_str(&content);
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        Ok(full_response)
    }

    /// Delete a workspace
    pub async fn delete_workspace(&self, workspace_id: &str) -> Result<(), String> {
        let url = format!("{}/chats/{}/settings", self.host, workspace_id);

        let mut request = self.http_client.delete(&url);

        if let Some(key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response = request.send().await
            .map_err(|e| format!("Failed to delete workspace: {}", e))?;

        if !response.status().is_success() && response.status().as_u16() != 404 {
            let error = response.text().await.unwrap_or_default();
            return Err(format!("Failed to delete workspace: {}", error));
        }

        Ok(())
    }
}

// ============================================================================
// DM Chat Integration
// ============================================================================

/// DM-specific chat workspace manager
pub struct DMChatManager {
    chat_client: MeilisearchChatClient,
    default_workspace: String,
}

impl DMChatManager {
    pub const DEFAULT_WORKSPACE: &'static str = "dm-assistant";

    pub fn new(host: &str, api_key: Option<&str>) -> Self {
        Self {
            chat_client: MeilisearchChatClient::new(host, api_key),
            default_workspace: Self::DEFAULT_WORKSPACE.to_string(),
        }
    }

    /// Initialize the DM chat workspace with appropriate settings
    pub async fn initialize(
        &self,
        llm_api_key: &str,
        model: Option<&str>,
        custom_system_prompt: Option<&str>,
    ) -> Result<(), String> {
        // Enable experimental chat feature
        self.chat_client.enable_chat_feature().await?;

        // Configure workspace
        let settings = ChatWorkspaceSettings {
            source: ChatLLMSource::OpenAi,
            api_key: Some(llm_api_key.to_string()),
            model: Some(model.unwrap_or("gpt-4o-mini").to_string()),
            prompts: Some(ChatPrompts {
                system: Some(
                    custom_system_prompt
                        .unwrap_or(DEFAULT_DM_SYSTEM_PROMPT)
                        .to_string()
                ),
                search_description: Some(
                    "Search the player's rulebooks, campaign notes, and lore for relevant information.".to_string()
                ),
                ..Default::default()
            }),
            base_url: None,
        };

        self.chat_client
            .configure_workspace(&self.default_workspace, &settings)
            .await?;

        log::info!("DM chat workspace initialized");
        Ok(())
    }

    /// Configure for Ollama (local LLM)
    pub async fn configure_for_ollama(
        &self,
        base_url: &str,
        model: &str,
        custom_system_prompt: Option<&str>,
    ) -> Result<(), String> {
        self.chat_client.enable_chat_feature().await?;

        let settings = ChatWorkspaceSettings {
            source: ChatLLMSource::VLlm, // vLLM compatible with Ollama API
            api_key: None,
            model: Some(model.to_string()),
            prompts: Some(ChatPrompts {
                system: Some(
                    custom_system_prompt
                        .unwrap_or(DEFAULT_DM_SYSTEM_PROMPT)
                        .to_string()
                ),
                ..Default::default()
            }),
            base_url: Some(base_url.to_string()),
        };

        self.chat_client
            .configure_workspace(&self.default_workspace, &settings)
            .await
    }

    /// Send a message to the DM and get a response
    pub async fn chat(&self, user_message: &str) -> Result<String, String> {
        let messages = vec![ChatMessage::user(user_message)];

        self.chat_client
            .chat_completion(&self.default_workspace, messages, "gpt-4o-mini")
            .await
    }

    /// Send a message with conversation history
    pub async fn chat_with_history(
        &self,
        messages: Vec<ChatMessage>,
        model: &str,
    ) -> Result<String, String> {
        self.chat_client
            .chat_completion(&self.default_workspace, messages, model)
            .await
    }

    /// Get streaming response
    pub async fn chat_stream(
        &self,
        user_message: &str,
        model: &str,
    ) -> Result<mpsc::Receiver<Result<String, String>>, String> {
        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages: vec![ChatMessage::user(user_message)],
            stream: true,
            temperature: Some(0.7),
            max_tokens: Some(2048),
        };

        self.chat_client
            .chat_completion_stream(&self.default_workspace, request)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_creation() {
        let user_msg = ChatMessage::user("Hello");
        assert_eq!(user_msg.role, "user");
        assert_eq!(user_msg.content, "Hello");

        let assistant_msg = ChatMessage::assistant("Hi there!");
        assert_eq!(assistant_msg.role, "assistant");
    }

    #[test]
    fn test_workspace_settings_default() {
        let settings = ChatWorkspaceSettings::default();
        assert!(settings.prompts.is_some());
    }
}
