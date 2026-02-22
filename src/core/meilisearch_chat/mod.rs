//! Meilisearch Chat Module
//!
//! Provides RAG-powered conversational search using Meilisearch's Chat API.
//! This enables the DM to have context-aware conversations that automatically
//! cite relevant documents (rules, lore, etc.) in responses.

mod client;
mod config;
mod prompts;
mod types;

// Re-export public types for external use
pub use client::MeilisearchChatClient;
pub use config::{
    ChatLLMSource, ChatPrompts, ChatProviderConfig, ChatWorkspaceSettings,
};
pub use prompts::{
    AZURE_DEFAULT_API_VERSION, AZURE_DEFAULT_DEPLOYMENT, COHERE_API_BASE_URL,
    COHERE_DEFAULT_MODEL, DEEPSEEK_API_BASE_URL, DEEPSEEK_DEFAULT_MODEL,
    DEFAULT_DM_SYSTEM_PROMPT, DEFAULT_SEARCH_DESCRIPTION, DEFAULT_SEARCH_INDEX_PARAM,
    DEFAULT_SEARCH_Q_PARAM, GOOGLE_API_BASE_URL, GOOGLE_DEFAULT_MODEL, GROK_API_BASE_URL,
    GROK_DEFAULT_MODEL, GROQ_API_BASE_URL, GROQ_DEFAULT_MODEL, OAUTH_PROXY_API_KEY_PLACEHOLDER,
    OLLAMA_API_KEY_PLACEHOLDER, OLLAMA_DEFAULT_HOST, OLLAMA_DEFAULT_MODEL,
    OPENROUTER_API_BASE_URL, TASK_COMPLETION_TIMEOUT_SECS, TOGETHER_API_BASE_URL,
};
pub use types::{
    AppendConversationMessageArgs, ChatCompletionRequest, ChatMessage, ChatProviderInfo,
    MeilisearchErrorDetail, MeilisearchErrorResponse, ParsedToolCall, SearchProgressArgs,
    SearchSourcesArgs, StreamChunk, StreamChoice, StreamDelta, ToolCallFunction, ToolCallInfo,
    get_meilisearch_chat_tools, list_chat_providers,
};

use tokio::sync::mpsc;

use crate::core::llm::providers::ProviderConfig;

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
        _model: Option<&str>,
        custom_system_prompt: Option<&str>,
    ) -> Result<(), String> {
        // Enable experimental chat feature
        self.chat_client.enable_chat_feature().await?;

        // Configure workspace
        let settings = ChatWorkspaceSettings {
            source: ChatLLMSource::OpenAi,
            api_key: Some(llm_api_key.to_string()),
            deployment_id: None,
            api_version: None,
            org_id: None,
            project_id: None,
            prompts: Some(ChatPrompts::with_system_prompt(
                custom_system_prompt.unwrap_or(DEFAULT_DM_SYSTEM_PROMPT)
            )),
            base_url: None,
        };

        self.chat_client
            .configure_workspace(&self.default_workspace, &settings)
            .await?;

        log::info!("DM chat workspace initialized with anti-filter prompts");
        Ok(())
    }

    /// Configure for Ollama (local LLM)
    pub async fn configure_for_ollama(
        &self,
        base_url: &str,
        _model: &str,
        custom_system_prompt: Option<&str>,
    ) -> Result<(), String> {
        self.chat_client.enable_chat_feature().await?;

        let settings = ChatWorkspaceSettings {
            source: ChatLLMSource::VLlm, // vLLM compatible with Ollama API
            api_key: None,
            deployment_id: None,
            api_version: None,
            org_id: None,
            project_id: None,
            prompts: Some(ChatPrompts::with_system_prompt(
                custom_system_prompt.unwrap_or(DEFAULT_DM_SYSTEM_PROMPT)
            )),
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
            tools: Some(get_meilisearch_chat_tools()),
        };

        self.chat_client
            .chat_completion_stream(&self.default_workspace, request)
            .await
    }

    /// Get streaming response with conversation history
    pub async fn chat_stream_with_history(
        &self,
        messages: Vec<ChatMessage>,
        model: &str,
    ) -> Result<mpsc::Receiver<Result<String, String>>, String> {
        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages,
            stream: true,
            temperature: Some(0.7),
            max_tokens: Some(2048),
            tools: Some(get_meilisearch_chat_tools()),
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
        let chat_config = ChatProviderConfig::try_from(config)
            .map_err(|e| e.to_string())?;

        // Create custom prompts with comprehensive anti-filter defaults
        let prompts = Some(ChatPrompts::with_system_prompt(
            custom_system_prompt.unwrap_or(DEFAULT_DM_SYSTEM_PROMPT)
        ));

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

// ============================================================================
// Tests
// ============================================================================

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
    }

    #[test]
    fn test_chat_llm_source_default() {
        let source = ChatLLMSource::default();
        assert_eq!(source, ChatLLMSource::OpenAi);
    }

    #[test]
    fn test_chat_prompts_default() {
        let prompts = ChatPrompts::default();
        // System prompt is None by default (set per-context)
        assert!(prompts.system.is_none());
        // Anti-filter search prompts are populated by default
        assert!(prompts.search_description.is_some());
        assert!(prompts.search_q_param.is_some());
        assert!(prompts.search_index_uid_param.is_some());
        // search_filter_param is None by default (let Meilisearch use its default)
        assert!(prompts.search_filter_param.is_none());
        // Verify the search_q_param contains anti-filter instructions
        let q_param = prompts.search_q_param.unwrap();
        assert!(q_param.contains("FORBIDDEN"));
        assert!(q_param.contains("Filter operators"));
    }

    #[test]
    fn test_chat_prompts_with_system() {
        let prompts = ChatPrompts::with_system_prompt("Custom system prompt");
        assert_eq!(prompts.system, Some("Custom system prompt".to_string()));
        // Should still have anti-filter defaults
        assert!(prompts.search_description.is_some());
        assert!(prompts.search_q_param.is_some());
    }

    #[test]
    fn test_chat_prompts_empty() {
        let prompts = ChatPrompts::empty();
        assert!(prompts.system.is_none());
        assert!(prompts.search_description.is_none());
        assert!(prompts.search_q_param.is_none());
    }

    #[test]
    fn test_dm_chat_manager_default_workspace() {
        assert_eq!(DMChatManager::DEFAULT_WORKSPACE, "dm-assistant");
    }

    #[test]
    fn test_get_meilisearch_chat_tools() {
        let tools = get_meilisearch_chat_tools();
        assert_eq!(tools.len(), 3);

        // Verify tool names
        let tool_names: Vec<&str> = tools.iter()
            .filter_map(|t| t.get("function")?.get("name")?.as_str())
            .collect();
        assert!(tool_names.contains(&"_meiliSearchProgress"));
        assert!(tool_names.contains(&"_meiliAppendConversationMessage"));
        assert!(tool_names.contains(&"_meiliSearchSources"));
    }

    #[test]
    fn test_parsed_tool_call_append_message() {
        let args_json = r#"{
            "role": "assistant",
            "content": null,
            "tool_calls": [{
                "id": "call_abc123",
                "type": "function",
                "function": {
                    "name": "_meiliSearchInIndex",
                    "arguments": "{\"index_uid\":\"docs\",\"q\":\"authentication\"}"
                }
            }],
            "tool_call_id": null
        }"#;

        let parsed = ParsedToolCall::parse("_meiliAppendConversationMessage", args_json);
        assert!(parsed.is_some());

        if let Some(ParsedToolCall::AppendMessage(args)) = parsed {
            assert_eq!(args.role, "assistant");
            assert!(args.content.is_none());
            assert!(args.tool_calls.is_some());
            let tool_calls = args.tool_calls.unwrap();
            assert_eq!(tool_calls.len(), 1);
            assert_eq!(tool_calls[0].id, "call_abc123");
            assert_eq!(tool_calls[0].function.name, "_meiliSearchInIndex");
        } else {
            panic!("Expected AppendMessage");
        }
    }

    #[test]
    fn test_parsed_tool_call_tool_result() {
        let args_json = r#"{
            "role": "tool",
            "content": "[{\"id\":\"1\",\"title\":\"Auth Guide\"}]",
            "tool_calls": null,
            "tool_call_id": "call_abc123"
        }"#;

        let parsed = ParsedToolCall::parse("_meiliAppendConversationMessage", args_json);
        assert!(parsed.is_some());

        if let Some(ParsedToolCall::AppendMessage(args)) = parsed {
            assert_eq!(args.role, "tool");
            assert!(args.content.is_some());
            assert_eq!(args.tool_call_id, Some("call_abc123".to_string()));
        } else {
            panic!("Expected AppendMessage");
        }
    }

    #[test]
    fn test_parsed_tool_call_search_progress() {
        let args_json = r#"{
            "call_id": "search_123",
            "function_name": "_meiliSearchInIndex",
            "function_parameters": "{\"index_uid\":\"documents\",\"q\":\"fireball spell\"}"
        }"#;

        let parsed = ParsedToolCall::parse("_meiliSearchProgress", args_json);
        assert!(parsed.is_some());

        if let Some(ParsedToolCall::SearchProgress(args)) = parsed {
            assert_eq!(args.call_id, "search_123");
            assert_eq!(args.function_name, "_meiliSearchInIndex");
            assert!(args.function_parameters.contains("fireball spell"));
        } else {
            panic!("Expected SearchProgress");
        }
    }

    #[test]
    fn test_parsed_tool_call_search_sources() {
        let args_json = r#"{
            "call_id": "search_123",
            "documents": [{"id": "1", "title": "PHB", "content": "Fireball..."}]
        }"#;

        let parsed = ParsedToolCall::parse("_meiliSearchSources", args_json);
        assert!(parsed.is_some());

        if let Some(ParsedToolCall::SearchSources(args)) = parsed {
            assert_eq!(args.call_id, "search_123");
            assert!(args.documents.is_array());
        } else {
            panic!("Expected SearchSources");
        }
    }

    #[test]
    fn test_parsed_tool_call_unknown() {
        let parsed = ParsedToolCall::parse("_unknownTool", "{}");
        assert!(parsed.is_some());

        if let Some(ParsedToolCall::Unknown { name, .. }) = parsed {
            assert_eq!(name, "_unknownTool");
        } else {
            panic!("Expected Unknown");
        }
    }
}
