//! Meilisearch Chat Module
//!
//! Provides RAG-powered conversational search using Meilisearch's Chat API.
//! This enables the DM to have context-aware conversations that automatically
//! cite relevant documents (rules, lore, etc.) in responses.

use serde::{Deserialize, Serialize};
use futures_util::StreamExt;
use tokio::sync::mpsc;

use super::llm::providers::ProviderConfig;

// ============================================================================
// Configuration Types
// ============================================================================

/// LLM provider source for Meilisearch Chat
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
// Chat Provider Configuration
// ============================================================================

/// Chat provider configuration for Meilisearch workspaces.
/// Maps the project's LLM providers to Meilisearch's chat sources.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ChatProviderConfig {
    /// OpenAI (native Meilisearch support)
    OpenAI {
        api_key: String,
        #[serde(default)]
        model: Option<String>,
        #[serde(default)]
        organization_id: Option<String>,
    },
    /// Anthropic Claude (via proxy)
    Claude {
        api_key: String,
        #[serde(default)]
        model: Option<String>,
        #[serde(default)]
        max_tokens: Option<u32>,
    },
    /// Mistral (native Meilisearch support)
    Mistral {
        api_key: String,
        #[serde(default)]
        model: Option<String>,
    },
    /// Ollama (via proxy as VLlm)
    Ollama {
        host: String,
        model: String,
    },
    /// Google Gemini (via proxy)
    Gemini {
        api_key: String,
        #[serde(default)]
        model: Option<String>,
    },
    /// OpenRouter (via proxy)
    OpenRouter {
        api_key: String,
        model: String,
    },
    /// Azure OpenAI (native Meilisearch support)
    AzureOpenAI {
        api_key: String,
        base_url: String,
        deployment_id: String,
        api_version: String,
    },
    /// Groq (via proxy)
    Groq {
        api_key: String,
        model: String,
    },
    /// Together.ai (via proxy)
    Together {
        api_key: String,
        model: String,
    },
    /// Cohere (via proxy)
    Cohere {
        api_key: String,
        model: String,
    },
    /// DeepSeek (via proxy)
    DeepSeek {
        api_key: String,
        model: String,
    },
    /// Claude Code CLI (via proxy, no API key needed)
    ClaudeCode {
        #[serde(default)]
        timeout_secs: Option<u64>,
        #[serde(default)]
        model: Option<String>,
    },
    /// Claude Desktop CDP (via proxy, no API key needed)
    ClaudeDesktop {
        #[serde(default)]
        port: Option<u16>,
        #[serde(default)]
        timeout_secs: Option<u64>,
    },
}

impl ChatProviderConfig {
    /// Get the provider ID for proxy routing
    pub fn provider_id(&self) -> &'static str {
        match self {
            ChatProviderConfig::OpenAI { .. } => "openai",
            ChatProviderConfig::Claude { .. } => "claude",
            ChatProviderConfig::Mistral { .. } => "mistral",
            ChatProviderConfig::Ollama { .. } => "ollama",
            ChatProviderConfig::Gemini { .. } => "gemini",
            ChatProviderConfig::OpenRouter { .. } => "openrouter",
            ChatProviderConfig::AzureOpenAI { .. } => "azure",
            ChatProviderConfig::Groq { .. } => "groq",
            ChatProviderConfig::Together { .. } => "together",
            ChatProviderConfig::Cohere { .. } => "cohere",
            ChatProviderConfig::DeepSeek { .. } => "deepseek",
            ChatProviderConfig::ClaudeCode { .. } => "claude-code",
            ChatProviderConfig::ClaudeDesktop { .. } => "claude-desktop",
        }
    }

    /// Check if this provider requires the proxy (vs native Meilisearch support)
    pub fn requires_proxy(&self) -> bool {
        !matches!(
            self,
            ChatProviderConfig::OpenAI { .. }
                | ChatProviderConfig::Mistral { .. }
                | ChatProviderConfig::AzureOpenAI { .. }
        )
    }

    /// Get the model identifier for proxy routing (format: provider:model)
    pub fn proxy_model_id(&self) -> String {
        let provider = self.provider_id();
        let model = match self {
            ChatProviderConfig::OpenAI { model, .. } => {
                model.as_deref().unwrap_or("gpt-4o-mini")
            }
            ChatProviderConfig::Claude { model, .. } => {
                model.as_deref().unwrap_or("claude-sonnet-4-20250514")
            }
            ChatProviderConfig::Mistral { model, .. } => {
                model.as_deref().unwrap_or("mistral-large-latest")
            }
            ChatProviderConfig::Ollama { model, .. } => model.as_str(),
            ChatProviderConfig::Gemini { model, .. } => {
                model.as_deref().unwrap_or("gemini-pro")
            }
            ChatProviderConfig::OpenRouter { model, .. } => model.as_str(),
            ChatProviderConfig::AzureOpenAI { .. } => "azure-deployment",
            ChatProviderConfig::Groq { model, .. } => model.as_str(),
            ChatProviderConfig::Together { model, .. } => model.as_str(),
            ChatProviderConfig::Cohere { model, .. } => model.as_str(),
            ChatProviderConfig::DeepSeek { model, .. } => model.as_str(),
            ChatProviderConfig::ClaudeCode { model, .. } => {
                model.as_deref().unwrap_or("claude-sonnet-4-20250514")
            }
            ChatProviderConfig::ClaudeDesktop { .. } => "claude-desktop",
        };
        format!("{}:{}", provider, model)
    }

    /// Convert to Meilisearch ChatWorkspaceSettings
    pub fn to_meilisearch_settings(&self, proxy_url: &str) -> ChatWorkspaceSettings {
        match self {
            // Native providers (direct to Meilisearch)
            ChatProviderConfig::OpenAI { api_key, model, .. } => ChatWorkspaceSettings {
                source: ChatLLMSource::OpenAi,
                api_key: Some(api_key.clone()),
                model: Some(model.as_deref().unwrap_or("gpt-4o-mini").to_string()),
                prompts: Some(ChatPrompts {
                    system: Some(DEFAULT_DM_SYSTEM_PROMPT.to_string()),
                    ..Default::default()
                }),
                base_url: None,
            },
            ChatProviderConfig::Mistral { api_key, model } => ChatWorkspaceSettings {
                source: ChatLLMSource::Mistral,
                api_key: Some(api_key.clone()),
                model: Some(model.as_deref().unwrap_or("mistral-large-latest").to_string()),
                prompts: Some(ChatPrompts {
                    system: Some(DEFAULT_DM_SYSTEM_PROMPT.to_string()),
                    ..Default::default()
                }),
                base_url: None,
            },
            ChatProviderConfig::AzureOpenAI { api_key, base_url, .. } => ChatWorkspaceSettings {
                source: ChatLLMSource::AzureOpenAi,
                api_key: Some(api_key.clone()),
                model: None, // Azure uses deployment_id
                prompts: Some(ChatPrompts {
                    system: Some(DEFAULT_DM_SYSTEM_PROMPT.to_string()),
                    ..Default::default()
                }),
                base_url: Some(base_url.clone()),
            },
            // All other providers route through proxy
            _ => ChatWorkspaceSettings {
                source: ChatLLMSource::VLlm,
                api_key: None, // Proxy handles auth
                model: Some(self.proxy_model_id()),
                prompts: Some(ChatPrompts {
                    system: Some(DEFAULT_DM_SYSTEM_PROMPT.to_string()),
                    ..Default::default()
                }),
                base_url: Some(format!("{}/v1", proxy_url)),
            },
        }
    }

    /// Convert to the project's ProviderConfig for proxy registration
    pub fn to_provider_config(&self) -> ProviderConfig {
        match self {
            ChatProviderConfig::OpenAI { api_key, model, organization_id, .. } => {
                ProviderConfig::OpenAI {
                    api_key: api_key.clone(),
                    model: model.as_deref().unwrap_or("gpt-4o-mini").to_string(),
                    max_tokens: 4096,
                    organization_id: organization_id.clone(),
                    base_url: None,
                }
            }
            ChatProviderConfig::Claude { api_key, model, max_tokens } => {
                ProviderConfig::Claude {
                    api_key: api_key.clone(),
                    model: model.as_deref().unwrap_or("claude-sonnet-4-20250514").to_string(),
                    max_tokens: max_tokens.unwrap_or(4096),
                }
            }
            ChatProviderConfig::Mistral { api_key, model } => ProviderConfig::Mistral {
                api_key: api_key.clone(),
                model: model.as_deref().unwrap_or("mistral-large-latest").to_string(),
            },
            ChatProviderConfig::Ollama { host, model } => ProviderConfig::Ollama {
                host: host.clone(),
                model: model.clone(),
            },
            ChatProviderConfig::Gemini { api_key, model } => ProviderConfig::Gemini {
                api_key: api_key.clone(),
                model: model.as_deref().unwrap_or("gemini-pro").to_string(),
            },
            ChatProviderConfig::OpenRouter { api_key, model } => ProviderConfig::OpenRouter {
                api_key: api_key.clone(),
                model: model.clone(),
            },
            ChatProviderConfig::AzureOpenAI { api_key, base_url, .. } => {
                ProviderConfig::OpenAI {
                    api_key: api_key.clone(),
                    model: "azure".to_string(),
                    max_tokens: 4096,
                    organization_id: None,
                    base_url: Some(base_url.clone()),
                }
            }
            ChatProviderConfig::Groq { api_key, model } => ProviderConfig::Groq {
                api_key: api_key.clone(),
                model: model.clone(),
            },
            ChatProviderConfig::Together { api_key, model } => ProviderConfig::Together {
                api_key: api_key.clone(),
                model: model.clone(),
            },
            ChatProviderConfig::Cohere { api_key, model } => ProviderConfig::Cohere {
                api_key: api_key.clone(),
                model: model.clone(),
            },
            ChatProviderConfig::DeepSeek { api_key, model } => ProviderConfig::DeepSeek {
                api_key: api_key.clone(),
                model: model.clone(),
            },
            ChatProviderConfig::ClaudeCode { timeout_secs, model } => ProviderConfig::ClaudeCode {
                timeout_secs: timeout_secs.unwrap_or(300),
                model: model.clone(),
                working_dir: None,
            },
            ChatProviderConfig::ClaudeDesktop { port, timeout_secs } => {
                ProviderConfig::ClaudeDesktop {
                    port: port.unwrap_or(9333),
                    timeout_secs: timeout_secs.unwrap_or(120),
                }
            }
        }
    }
}

/// Information about available chat providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatProviderInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub requires_api_key: bool,
    pub is_native: bool,
}

/// Get information about all available chat providers
pub fn list_chat_providers() -> Vec<ChatProviderInfo> {
    vec![
        ChatProviderInfo {
            id: "openai",
            name: "OpenAI",
            description: "GPT-4o, GPT-4, GPT-3.5 models",
            requires_api_key: true,
            is_native: true,
        },
        ChatProviderInfo {
            id: "claude",
            name: "Anthropic Claude",
            description: "Claude 3.5 Sonnet, Claude 3 Opus/Haiku",
            requires_api_key: true,
            is_native: false,
        },
        ChatProviderInfo {
            id: "mistral",
            name: "Mistral AI",
            description: "Mistral Large, Codestral, Mixtral",
            requires_api_key: true,
            is_native: true,
        },
        ChatProviderInfo {
            id: "ollama",
            name: "Ollama (Local)",
            description: "Run open models locally",
            requires_api_key: false,
            is_native: false,
        },
        ChatProviderInfo {
            id: "gemini",
            name: "Google Gemini",
            description: "Gemini Pro, Gemini Ultra",
            requires_api_key: true,
            is_native: false,
        },
        ChatProviderInfo {
            id: "openrouter",
            name: "OpenRouter",
            description: "Access many models via single API",
            requires_api_key: true,
            is_native: false,
        },
        ChatProviderInfo {
            id: "azure",
            name: "Azure OpenAI",
            description: "Azure-hosted OpenAI models",
            requires_api_key: true,
            is_native: true,
        },
        ChatProviderInfo {
            id: "groq",
            name: "Groq",
            description: "Fast inference with Llama, Mixtral",
            requires_api_key: true,
            is_native: false,
        },
        ChatProviderInfo {
            id: "together",
            name: "Together.ai",
            description: "Open models at scale",
            requires_api_key: true,
            is_native: false,
        },
        ChatProviderInfo {
            id: "cohere",
            name: "Cohere",
            description: "Command R+, Command models",
            requires_api_key: true,
            is_native: false,
        },
        ChatProviderInfo {
            id: "deepseek",
            name: "DeepSeek",
            description: "DeepSeek Coder, DeepSeek Chat",
            requires_api_key: true,
            is_native: false,
        },
        ChatProviderInfo {
            id: "claude-code",
            name: "Claude Code CLI",
            description: "Uses existing Claude Code authentication",
            requires_api_key: false,
            is_native: false,
        },
        ChatProviderInfo {
            id: "claude-desktop",
            name: "Claude Desktop",
            description: "Uses existing Claude Desktop app",
            requires_api_key: false,
            is_native: false,
        },
    ]
}

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

    /// Configure a workspace with a specific chat provider
    ///
    /// This is a convenience method that converts a ChatProviderConfig to
    /// Meilisearch settings and configures the workspace.
    ///
    /// # Arguments
    /// * `workspace_id` - The workspace identifier
    /// * `provider` - The chat provider configuration
    /// * `proxy_url` - URL of the LLM proxy (for non-native providers)
    /// * `custom_prompts` - Optional custom prompts to override defaults
    pub async fn configure_workspace_with_provider(
        &self,
        workspace_id: &str,
        provider: &ChatProviderConfig,
        proxy_url: &str,
        custom_prompts: Option<ChatPrompts>,
    ) -> Result<(), String> {
        // First ensure chat feature is enabled
        self.enable_chat_feature().await?;

        // Convert provider config to Meilisearch settings
        let mut settings = provider.to_meilisearch_settings(proxy_url);

        // Apply custom prompts if provided
        if let Some(prompts) = custom_prompts {
            settings.prompts = Some(prompts);
        }

        // Configure the workspace
        self.configure_workspace(workspace_id, &settings).await?;

        log::info!(
            "Configured workspace '{}' with provider: {} (native: {})",
            workspace_id,
            provider.provider_id(),
            !provider.requires_proxy()
        );

        Ok(())
    }

    /// Get the host URL
    pub fn host(&self) -> &str {
        &self.host
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

    /// Configure from LLM ProviderConfig (convenience method for default workspace)
    pub async fn configure_from_provider_config(
        &self,
        config: &ProviderConfig,
        proxy_url: &str,
        custom_system_prompt: Option<&str>,
    ) -> Result<(), String> {
        const DEFAULT_WORKSPACE: &str = "dm-assistant";
        const DEFAULT_DM_PROMPT: &str = "You are a knowledgeable and creative Dungeon Master assistant.";

        let chat_config = match config {
            ProviderConfig::OpenAI { api_key, model, organization_id, .. } => ChatProviderConfig::OpenAI {
                api_key: api_key.clone(),
                model: Some(model.clone()),
                organization_id: organization_id.clone(),
            },
            ProviderConfig::Claude { api_key, model, max_tokens } => ChatProviderConfig::Claude {
                api_key: api_key.clone(),
                model: Some(model.clone()),
                max_tokens: Some(*max_tokens),
            },
            ProviderConfig::Mistral { api_key, model } => ChatProviderConfig::Mistral {
                api_key: api_key.clone(),
                model: Some(model.clone()),
            },
            ProviderConfig::Ollama { host, model } => ChatProviderConfig::Ollama {
                host: host.clone(),
                model: model.clone(),
            },
            ProviderConfig::Gemini { api_key, model } => ChatProviderConfig::Gemini {
                api_key: api_key.clone(),
                model: Some(model.clone()),
            },
            ProviderConfig::OpenRouter { api_key, model } => ChatProviderConfig::OpenRouter {
                api_key: api_key.clone(),
                model: model.clone(),
            },
            ProviderConfig::Groq { api_key, model } => ChatProviderConfig::Groq {
                api_key: api_key.clone(),
                model: model.clone(),
            },
            ProviderConfig::Together { api_key, model } => ChatProviderConfig::Together {
                api_key: api_key.clone(),
                model: model.clone(),
            },
            ProviderConfig::Cohere { api_key, model } => ChatProviderConfig::Cohere {
                api_key: api_key.clone(),
                model: model.clone(),
            },
            ProviderConfig::DeepSeek { api_key, model } => ChatProviderConfig::DeepSeek {
                api_key: api_key.clone(),
                model: model.clone(),
            },
            ProviderConfig::ClaudeCode { timeout_secs, model, .. } => ChatProviderConfig::ClaudeCode {
                timeout_secs: Some(*timeout_secs),
                model: model.clone(),
            },
            ProviderConfig::ClaudeDesktop { port, timeout_secs } => ChatProviderConfig::ClaudeDesktop {
                port: Some(*port),
                timeout_secs: Some(*timeout_secs),
            },
            ProviderConfig::GeminiCli { .. } => return Err("Gemini CLI not supported for Meilisearch chat yet".to_string()),
            ProviderConfig::Meilisearch { .. } => return Err("Recursive Meilisearch configuration".to_string()),
        };

        let prompts = Some(ChatPrompts {
            system: Some(
                custom_system_prompt
                    .unwrap_or(DEFAULT_DM_PROMPT)
                    .to_string()
            ),
            ..Default::default()
        });

        self.configure_workspace_with_provider(
            DEFAULT_WORKSPACE,
            &chat_config,
            proxy_url,
            prompts
        ).await
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

    /// Configure from LLM ProviderConfig
    pub async fn configure_from_provider_config(
        &self,
        config: &ProviderConfig,
        proxy_url: &str,
        custom_system_prompt: Option<&str>,
    ) -> Result<(), String> {
        let chat_config = match config {
            ProviderConfig::OpenAI { api_key, model, organization_id, .. } => ChatProviderConfig::OpenAI {
                api_key: api_key.clone(),
                model: Some(model.clone()),
                organization_id: organization_id.clone(),
            },
            ProviderConfig::Claude { api_key, model, max_tokens } => ChatProviderConfig::Claude {
                api_key: api_key.clone(),
                model: Some(model.clone()),
                max_tokens: Some(*max_tokens),
            },
            ProviderConfig::Mistral { api_key, model } => ChatProviderConfig::Mistral {
                api_key: api_key.clone(),
                model: Some(model.clone()),
            },
            ProviderConfig::Ollama { host, model } => ChatProviderConfig::Ollama {
                host: host.clone(),
                model: model.clone(),
            },
            ProviderConfig::Gemini { api_key, model } => ChatProviderConfig::Gemini {
                api_key: api_key.clone(),
                model: Some(model.clone()),
            },
            ProviderConfig::OpenRouter { api_key, model } => ChatProviderConfig::OpenRouter {
                api_key: api_key.clone(),
                model: model.clone(),
            },
            // Note: ProviderConfig doesn't have AzureOpenAI variant yet in mod.rs,
            // but ChatProviderConfig does. We skip it or map if it exists.
            // Based on view_file output of mod.rs, AzureOpenAI is NOT in ProviderConfig.
            // So we handle other variants.

            ProviderConfig::Groq { api_key, model } => ChatProviderConfig::Groq {
                api_key: api_key.clone(),
                model: model.clone(),
            },
            ProviderConfig::Together { api_key, model } => ChatProviderConfig::Together {
                api_key: api_key.clone(),
                model: model.clone(),
            },
            ProviderConfig::Cohere { api_key, model } => ChatProviderConfig::Cohere {
                api_key: api_key.clone(),
                model: model.clone(),
            },
            ProviderConfig::DeepSeek { api_key, model } => ChatProviderConfig::DeepSeek {
                api_key: api_key.clone(),
                model: model.clone(),
            },
            ProviderConfig::ClaudeCode { timeout_secs, model, .. } => ChatProviderConfig::ClaudeCode {
                timeout_secs: Some(*timeout_secs),
                model: model.clone(),
            },
            ProviderConfig::ClaudeDesktop { port, timeout_secs } => ChatProviderConfig::ClaudeDesktop {
                port: Some(*port),
                timeout_secs: Some(*timeout_secs),
            },
            // Handle GeminiCLI as generic or unsupported for now if no direct map
            ProviderConfig::GeminiCli { .. } => return Err("Gemini CLI not supported for Meilisearch chat yet".to_string()),

            // Meilisearch provider is for using Meilisearch as a provider, creating a loop if we configure it here
            ProviderConfig::Meilisearch { .. } => return Err("Recursive Meilisearch configuration".to_string()),
        };

        // Create custom prompts object
        let prompts = Some(ChatPrompts {
            system: Some(
                custom_system_prompt
                    .unwrap_or(DEFAULT_DM_SYSTEM_PROMPT)
                    .to_string()
            ),
            ..Default::default()
        });

        self.chat_client
            .configure_workspace_with_provider(
                &self.default_workspace,
                &chat_config,
                proxy_url,
                prompts
            )
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

    #[test]
    fn test_chat_provider_config_provider_id() {
        let openai = ChatProviderConfig::OpenAI {
            api_key: "test".to_string(),
            model: Some("gpt-4".to_string()),
            organization_id: None,
        };
        assert_eq!(openai.provider_id(), "openai");

        let claude = ChatProviderConfig::Claude {
            api_key: "test".to_string(),
            model: Some("claude-3".to_string()),
            max_tokens: Some(4096),
        };
        assert_eq!(claude.provider_id(), "claude");

        let ollama = ChatProviderConfig::Ollama {
            host: "http://localhost:11434".to_string(),
            model: "llama2".to_string(),
        };
        assert_eq!(ollama.provider_id(), "ollama");
    }

    #[test]
    fn test_chat_provider_config_creation() {
        // Test that all variants can be created
        let openai = ChatProviderConfig::OpenAI {
            api_key: "test".to_string(),
            model: Some("gpt-4o".to_string()),
            organization_id: None,
        };
        assert_eq!(openai.provider_id(), "openai");

        let azure = ChatProviderConfig::AzureOpenAI {
            api_key: "test".to_string(),
            base_url: "https://example.openai.azure.com".to_string(),
            deployment_id: "gpt-4".to_string(),
            api_version: "2023-05-15".to_string(),
        };
        assert_eq!(azure.provider_id(), "azure");

        let claude_code = ChatProviderConfig::ClaudeCode {
            timeout_secs: Some(120),
            model: Some("claude-3-sonnet".to_string()),
        };
        assert_eq!(claude_code.provider_id(), "claude-code");
    }

    #[test]
    fn test_chat_llm_source_default() {
        let source = ChatLLMSource::default();
        assert_eq!(source, ChatLLMSource::OpenAi);
    }

    #[test]
    fn test_chat_prompts_default() {
        let prompts = ChatPrompts::default();
        assert!(prompts.system.is_none());
        assert!(prompts.search_description.is_none());
    }

    #[test]
    fn test_dm_chat_manager_default_workspace() {
        assert_eq!(DMChatManager::DEFAULT_WORKSPACE, "dm-assistant");
    }
}
