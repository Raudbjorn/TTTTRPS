//! Meilisearch Chat Module
//!
//! Provides RAG-powered conversational search using Meilisearch's Chat API.
//! This enables the DM to have context-aware conversations that automatically
//! cite relevant documents (rules, lore, etc.) in responses.

use serde::{Deserialize, Serialize};
use futures_util::StreamExt;
use tokio::sync::mpsc;

use super::llm::model_selector::model_selector;
use super::llm::providers::ProviderConfig;

// ============================================================================
// Anti-Filter Hallucination Prompts (must be before ChatPrompts)
// ============================================================================
// These prompts are designed to prevent LLMs from generating filter syntax
// that Meilisearch cannot process, which causes invalid_search_filter errors.

/// Default search description - tells LLM when/how to use search
pub const DEFAULT_SEARCH_DESCRIPTION: &str = r#"Search the TTRPG knowledge base for rules, lore, creatures, spells, and game content.

WHEN TO SEARCH:
- User asks about game mechanics, rules, or stats
- User asks about specific creatures, spells, items, or characters
- User needs information from rulebooks or source materials
- You need to cite sources or verify information

DO NOT SEARCH FOR:
- Greetings or casual conversation
- Questions you can answer from conversation context
- Creative content generation (unless researching source material first)"#;

/// Default search query parameter prompt - CRITICAL for preventing filter errors
pub const DEFAULT_SEARCH_Q_PARAM: &str = r#"Generate a simple keyword search query using 2-6 relevant terms.

RULES:
1. Use ONLY plain keywords separated by spaces
2. Include specific names, terms, and concepts from the question
3. Prioritize unique/specific terms over generic ones

FORBIDDEN - NEVER USE:
- Filter operators: = != > < >= <= AND OR NOT IN TO
- Field syntax: field:value, field=value, category:X
- SQL syntax: WHERE, SELECT, LIKE, IS NULL, IS NOT NULL
- Regex operators: =~ !~ * ? [ ]
- Quotes for exact matching: "exact phrase"
- Boolean operators: && || !

EXAMPLES:
✓ "goblin stat block challenge rating"
✓ "fireball spell damage evocation"
✓ "Delta Green agent character creation"
✗ "type = monster AND cr > 5"
✗ "category:spell school:evocation"
✗ "name =~ 'dragon.*'"#;

/// Default index selection prompt
pub const DEFAULT_SEARCH_INDEX_PARAM: &str = r#"Select which index to search.

AVAILABLE INDEXES:
- 'documents': Primary index containing all uploaded PDFs, rulebooks, and source materials. USE THIS FOR MOST QUERIES.

RULES:
- ALWAYS use 'documents' for rules, lore, creatures, spells, items
- NEVER invent index names or use the topic as an index name
- When in doubt, use 'documents'"#;

// ============================================================================
// Meilisearch Chat Tools
// ============================================================================

/// Tool definitions for Meilisearch chat requests.
/// These tools enable RAG functionality and conversation context management.
pub fn get_meilisearch_chat_tools() -> Vec<serde_json::Value> {
    vec![
        // Progress reporting tool
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "_meiliSearchProgress",
                "description": "Provides information about the current Meilisearch search operation",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "call_id": {
                            "type": "string",
                            "description": "The call ID to track the sources of the search"
                        },
                        "function_name": {
                            "type": "string",
                            "description": "The name of the function we are executing"
                        },
                        "function_parameters": {
                            "type": "string",
                            "description": "The parameters of the function we are executing, encoded in JSON"
                        }
                    },
                    "required": ["call_id", "function_name", "function_parameters"],
                    "additionalProperties": false
                },
                "strict": true
            }
        }),
        // Conversation context management tool
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "_meiliAppendConversationMessage",
                "description": "Append a new message to the conversation based on what happened internally. Used to maintain conversation context for stateless chat.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "role": {
                            "type": "string",
                            "description": "The role of the message author: 'assistant' or 'tool'"
                        },
                        "content": {
                            "type": ["string", "null"],
                            "description": "The contents of the message. Required unless tool_calls is specified."
                        },
                        "tool_calls": {
                            "type": ["array", "null"],
                            "description": "The tool calls generated by the model",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "function": {
                                        "type": "object",
                                        "description": "The function that the model called",
                                        "properties": {
                                            "name": {
                                                "type": "string",
                                                "description": "The name of the function to call"
                                            },
                                            "arguments": {
                                                "type": "string",
                                                "description": "The arguments to call the function with, as JSON"
                                            }
                                        }
                                    },
                                    "id": {
                                        "type": "string",
                                        "description": "The ID of the tool call"
                                    },
                                    "type": {
                                        "type": "string",
                                        "description": "The type of the tool (currently only 'function')"
                                    }
                                }
                            }
                        },
                        "tool_call_id": {
                            "type": ["string", "null"],
                            "description": "Tool call ID that this message is responding to"
                        }
                    },
                    "required": ["role", "content", "tool_calls", "tool_call_id"],
                    "additionalProperties": false
                },
                "strict": true
            }
        }),
        // Source documents tool
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "_meiliSearchSources",
                "description": "Provides source documents from the search results",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "call_id": {
                            "type": "string",
                            "description": "The call ID to track the original search associated to those sources"
                        },
                        "documents": {
                            "type": "object",
                            "description": "The documents associated with the search. Only displayed attributes are returned."
                        }
                    },
                    "required": ["call_id", "documents"],
                    "additionalProperties": false
                },
                "strict": true
            }
        })
    ]
}

// ============================================================================
// Tool Call Types for Conversation Context
// ============================================================================

/// Arguments for _meiliAppendConversationMessage tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendConversationMessageArgs {
    /// Message role: "assistant" or "tool"
    pub role: String,
    /// Message content (for tool results, may be null for assistant tool_calls)
    pub content: Option<String>,
    /// Tool calls made by the assistant
    pub tool_calls: Option<Vec<ToolCallInfo>>,
    /// Tool call ID this message responds to
    pub tool_call_id: Option<String>,
}

/// Tool call information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallInfo {
    /// Unique ID for this tool call
    pub id: String,
    /// Type of tool (always "function" currently)
    #[serde(rename = "type")]
    pub call_type: String,
    /// Function details
    pub function: ToolCallFunction,
}

/// Function details within a tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    /// Function name
    pub name: String,
    /// Function arguments as JSON string
    pub arguments: String,
}

/// Arguments for _meiliSearchProgress tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchProgressArgs {
    /// Call ID to track this search
    pub call_id: String,
    /// Function being executed
    pub function_name: String,
    /// Function parameters as JSON string
    pub function_parameters: String,
}

/// Arguments for _meiliSearchSources tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSourcesArgs {
    /// Call ID linking to the original search
    pub call_id: String,
    /// Source documents
    pub documents: serde_json::Value,
}

/// Parsed tool call from stream
#[derive(Debug, Clone)]
pub enum ParsedToolCall {
    /// Conversation message to append
    AppendMessage(AppendConversationMessageArgs),
    /// Search progress update
    SearchProgress(SearchProgressArgs),
    /// Search sources/documents
    SearchSources(SearchSourcesArgs),
    /// Unknown tool call
    Unknown { name: String, arguments: String },
}

impl ParsedToolCall {
    /// Parse a tool call from name and arguments JSON
    pub fn parse(name: &str, arguments: &str) -> Option<Self> {
        match name {
            "_meiliAppendConversationMessage" => {
                serde_json::from_str::<AppendConversationMessageArgs>(arguments)
                    .ok()
                    .map(ParsedToolCall::AppendMessage)
            }
            "_meiliSearchProgress" => {
                serde_json::from_str::<SearchProgressArgs>(arguments)
                    .ok()
                    .map(ParsedToolCall::SearchProgress)
            }
            "_meiliSearchSources" => {
                serde_json::from_str::<SearchSourcesArgs>(arguments)
                    .ok()
                    .map(ParsedToolCall::SearchSources)
            }
            _ => Some(ParsedToolCall::Unknown {
                name: name.to_string(),
                arguments: arguments.to_string(),
            }),
        }
    }
}

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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
    /// Description of the filter parameter for the AI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_filter_param: Option<String>,
}

impl Default for ChatPrompts {
    /// Creates ChatPrompts with anti-filter hallucination defaults.
    /// These prompts explicitly instruct the LLM to use only keyword searches
    /// and forbid filter syntax that causes Meilisearch errors.
    fn default() -> Self {
        Self {
            system: None, // System prompt is set separately based on context
            search_description: Some(DEFAULT_SEARCH_DESCRIPTION.to_string()),
            search_q_param: Some(DEFAULT_SEARCH_Q_PARAM.to_string()),
            search_index_uid_param: Some(DEFAULT_SEARCH_INDEX_PARAM.to_string()),
            search_filter_param: None, // Let Meilisearch use default filter description
        }
    }
}

impl ChatPrompts {
    /// Create ChatPrompts with a custom system prompt but default anti-filter search prompts
    pub fn with_system_prompt(system_prompt: &str) -> Self {
        Self {
            system: Some(system_prompt.to_string()),
            ..Default::default()
        }
    }

    /// Create empty ChatPrompts (all fields None)
    pub fn empty() -> Self {
        Self {
            system: None,
            search_description: None,
            search_q_param: None,
            search_index_uid_param: None,
            search_filter_param: None,
        }
    }
}

/// Workspace settings for Meilisearch Chat
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatWorkspaceSettings {
    /// LLM provider source
    pub source: ChatLLMSource,
    /// API key for the LLM provider
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Azure OpenAI deployment ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_id: Option<String>,
    /// Azure OpenAI API version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
    /// Azure OpenAI Organization ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_id: Option<String>,
    /// Azure OpenAI Project ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
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
            deployment_id: None,
            api_version: None,
            org_id: None,
            project_id: None,
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
Use the search tool to find relevant information before answering.
VALID INDEXES:
- `documents`: User uploaded files (PDFs, etc.)
- `rules`: Game mechanics and rulebooks
- `fiction`: Lore and narrative content
- `chat`: Conversation history

Do NOT invent index names. Only use the ones listed above."#;

/// Default model for Grok/xAI provider
pub const GROK_DEFAULT_MODEL: &str = "grok-3-mini";

/// Base URL for Grok/xAI API (OpenAI-compatible)
pub const GROK_API_BASE_URL: &str = "https://api.x.ai/v1";

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
    /// Grok/xAI (OpenAI-compatible, native via VLlm source with xAI base URL)
    Grok {
        api_key: String,
        #[serde(default)]
        model: Option<String>,
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
    /// Claude Gate OAuth (via proxy, no API key needed - uses OAuth tokens)
    ClaudeGate {
        model: String,
        #[serde(default)]
        max_tokens: Option<u32>,
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
            ChatProviderConfig::Grok { .. } => "grok",
            ChatProviderConfig::ClaudeCode { .. } => "claude-code",
            ChatProviderConfig::ClaudeDesktop { .. } => "claude-desktop",
            ChatProviderConfig::ClaudeGate { .. } => "claude-gate",
        }
    }

    /// Check if this provider requires the proxy (vs native Meilisearch support)
    pub fn requires_proxy(&self) -> bool {
        !matches!(
            self,
            ChatProviderConfig::OpenAI { .. }
                | ChatProviderConfig::Mistral { .. }
                | ChatProviderConfig::AzureOpenAI { .. }
                | ChatProviderConfig::Grok { .. }  // OpenAI-compatible
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
                return format!(
                    "claude:{}",
                    model.clone().unwrap_or_else(|| model_selector().select_model_sync())
                );
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
            ChatProviderConfig::Grok { model, .. } => {
                model.as_deref().unwrap_or(GROK_DEFAULT_MODEL)
            }
            ChatProviderConfig::ClaudeCode { model, .. } => {
                return format!(
                    "claude-code:{}",
                    model.clone().unwrap_or_else(|| model_selector().select_model_sync())
                );
            }
            ChatProviderConfig::ClaudeDesktop { .. } => "claude-desktop",
            ChatProviderConfig::ClaudeGate { model, .. } => {
                return format!("claude-gate:{}", model);
            }
        };
        format!("{}:{}", provider, model)
    }

    /// Convert to Meilisearch ChatWorkspaceSettings
    pub fn to_meilisearch_settings(&self, proxy_url: &str) -> ChatWorkspaceSettings {
        match self {
            // Native providers (direct to Meilisearch)
            ChatProviderConfig::OpenAI { api_key, .. } => ChatWorkspaceSettings {
                source: ChatLLMSource::OpenAi,
                api_key: Some(api_key.clone()),
                deployment_id: None,
                api_version: None,
                org_id: None,
                project_id: None,
                prompts: Some(ChatPrompts {
                    system: Some(DEFAULT_DM_SYSTEM_PROMPT.to_string()),
                    ..Default::default()
                }),
                base_url: None,
            },
            ChatProviderConfig::Mistral { api_key, .. } => ChatWorkspaceSettings {
                source: ChatLLMSource::Mistral,
                api_key: Some(api_key.clone()),
                deployment_id: None,
                api_version: None,
                org_id: None,
                project_id: None,
                prompts: Some(ChatPrompts {
                    system: Some(DEFAULT_DM_SYSTEM_PROMPT.to_string()),
                    ..Default::default()
                }),
                base_url: None,
            },
            ChatProviderConfig::Gemini { api_key, .. } => ChatWorkspaceSettings {
                source: ChatLLMSource::Gemini,
                api_key: Some(api_key.clone()),
                deployment_id: None,
                api_version: None,
                org_id: None,
                project_id: None,
                prompts: Some(ChatPrompts {
                    system: Some(DEFAULT_DM_SYSTEM_PROMPT.to_string()),
                    ..Default::default()
                }),
                base_url: None,
            },
            ChatProviderConfig::Ollama { host, .. } => ChatWorkspaceSettings {
                source: ChatLLMSource::VLlm,
                api_key: Some("ollama".to_string()), // Placeholder key required by Meilisearch for vLLM source
                deployment_id: None,
                api_version: None,
                org_id: None,
                project_id: None,
                prompts: Some(ChatPrompts {
                    system: Some(DEFAULT_DM_SYSTEM_PROMPT.to_string()),
                    ..Default::default()
                }),
                base_url: Some(format!("{}/v1", host.trim_end_matches('/'))),
            },
            ChatProviderConfig::AzureOpenAI { api_key, base_url, deployment_id, api_version, .. } => ChatWorkspaceSettings {
                source: ChatLLMSource::AzureOpenAi,
                api_key: Some(api_key.clone()),
                deployment_id: Some(deployment_id.clone()),
                api_version: Some(api_version.clone()),
                org_id: None, // Not currently stored in config
                project_id: None, // Not currently stored in config
                prompts: Some(ChatPrompts {
                    system: Some(DEFAULT_DM_SYSTEM_PROMPT.to_string()),
                    ..Default::default()
                }),
                base_url: Some(base_url.clone()),
            },
            // Grok/xAI - OpenAI-compatible, uses VLlm source with xAI base URL
            ChatProviderConfig::Grok { api_key, .. } => ChatWorkspaceSettings {
                source: ChatLLMSource::VLlm,
                api_key: Some(api_key.clone()),
                deployment_id: None,
                api_version: None,
                org_id: None,
                project_id: None,
                prompts: Some(ChatPrompts {
                    system: Some(DEFAULT_DM_SYSTEM_PROMPT.to_string()),
                    ..Default::default()
                }),
                base_url: Some(GROK_API_BASE_URL.to_string()),
            },
            // All other providers route through proxy
            _ => ChatWorkspaceSettings {
                source: ChatLLMSource::VLlm,
                api_key: None, // Proxy handles auth
                deployment_id: None,
                api_version: None,
                org_id: None,
                project_id: None,
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
            // Grok/xAI uses OpenAI-compatible API
            ChatProviderConfig::Grok { api_key, model } => ProviderConfig::OpenAI {
                api_key: api_key.clone(),
                model: model.as_deref().unwrap_or(GROK_DEFAULT_MODEL).to_string(),
                max_tokens: 4096,
                organization_id: None,
                base_url: Some(GROK_API_BASE_URL.to_string()),
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
            ChatProviderConfig::ClaudeGate { model, max_tokens } => {
                ProviderConfig::ClaudeGate {
                    storage_backend: "auto".to_string(),
                    model: model.clone(),
                    max_tokens: max_tokens.unwrap_or(8192),
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
            id: "grok",
            name: "Grok (xAI)",
            description: "Grok models from xAI",
            requires_api_key: true,
            is_native: true,  // OpenAI-compatible, no proxy needed
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<serde_json::Value>>,
}

/// Streaming response delta
#[derive(Debug, Clone, Deserialize)]
pub struct StreamDelta {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub tool_calls: Option<serde_json::Value>,
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

#[derive(Debug, Clone, Deserialize)]
pub struct StreamChunk {
    pub id: String,
    pub choices: Vec<StreamChoice>,
    #[serde(default)]
    pub model: Option<String>,
}

/// Error response from Meilisearch
#[derive(Debug, Clone, Deserialize)]
pub struct MeilisearchErrorResponse {
    pub error: MeilisearchErrorDetail,
    #[serde(rename = "type")]
    pub error_type: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MeilisearchErrorDetail {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: String,
    pub code: Option<String>,
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
                "chatCompletions": true
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

            log::info!("Starting SSE stream processing");

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        let chunk_str = String::from_utf8_lossy(&bytes);
                        log::debug!("Received chunk: {}", chunk_str);
                        buffer.push_str(&chunk_str);

                        // Process complete SSE events
                        while let Some(pos) = buffer.find("\n\n") {
                            let event = buffer[..pos].to_string();
                            buffer = buffer[pos + 2..].to_string();

                            log::debug!("Processing event: {}", event);

                            // Parse SSE event
                            for line in event.lines() {
                                if line.starts_with("data: ") {
                                    let data = &line[6..];
                                    if data == "[DONE]" {
                                        log::info!("Stream finished with [DONE]");
                                        let _ = tx.send(Ok("[DONE]".to_string())).await;
                                        return;
                                    }

                                    // Parse JSON chunk
                                    match serde_json::from_str::<StreamChunk>(data) {
                                        Ok(chunk) => {
                                            for choice in chunk.choices {
                                                if let Some(content) = choice.delta.content {
                                                    // Filter out tool call JSON that models output as text
                                                    // when they don't support structured tool calling
                                                    let trimmed = content.trim();
                                                    let is_tool_call_json = trimmed.starts_with('{')
                                                        && trimmed.contains("\"name\"")
                                                        && (trimmed.contains("_meili") || trimmed.contains("_search"));

                                                    if is_tool_call_json {
                                                        log::debug!("Filtering tool call JSON from content: {}", content);
                                                    } else {
                                                        log::debug!("Emitting content: {}", content);
                                                        let _ = tx.send(Ok(content)).await;
                                                    }
                                                } else if let Some(tool_calls) = choice.delta.tool_calls {
                                                    // Process tool calls from Meilisearch
                                                    // These include _meiliAppendConversationMessage for context management
                                                    if let Some(tool_array) = tool_calls.as_array() {
                                                        for tool_call in tool_array {
                                                            if let Some(func) = tool_call.get("function") {
                                                                let func_name = func.get("name")
                                                                    .and_then(|n| n.as_str())
                                                                    .unwrap_or("");
                                                                let func_args = func.get("arguments")
                                                                    .and_then(|a| a.as_str())
                                                                    .unwrap_or("{}");

                                                                // Parse and handle the tool call
                                                                match ParsedToolCall::parse(func_name, func_args) {
                                                                    Some(ParsedToolCall::AppendMessage(args)) => {
                                                                        // _meiliAppendConversationMessage: track conversation context
                                                                        // Emit as structured JSON for frontend to maintain history
                                                                        log::debug!("Conversation append: role={}, has_tool_calls={}, tool_call_id={:?}",
                                                                            args.role,
                                                                            args.tool_calls.is_some(),
                                                                            args.tool_call_id);

                                                                        // Emit as special event for frontend conversation management
                                                                        if let Ok(json) = serde_json::to_string(&serde_json::json!({
                                                                            "_type": "conversation_append",
                                                                            "role": args.role,
                                                                            "content": args.content,
                                                                            "tool_calls": args.tool_calls,
                                                                            "tool_call_id": args.tool_call_id
                                                                        })) {
                                                                            let _ = tx.send(Ok(format!("[MEILI_CONTEXT:{}]", json))).await;
                                                                        }
                                                                    }
                                                                    Some(ParsedToolCall::SearchProgress(args)) => {
                                                                        // _meiliSearchProgress: emit for UI progress display
                                                                        log::debug!("Search progress: {} - {:?}", args.function_name, args.function_parameters);

                                                                        // Parse function_parameters to extract search query
                                                                        if let Ok(params) = serde_json::from_str::<serde_json::Value>(&args.function_parameters) {
                                                                            if let Some(q) = params.get("q").and_then(|q| q.as_str()) {
                                                                                let _ = tx.send(Ok(format!("[MEILI_SEARCH:{}]", q))).await;
                                                                            }
                                                                        }
                                                                    }
                                                                    Some(ParsedToolCall::SearchSources(args)) => {
                                                                        // _meiliSearchSources: emit for citation display
                                                                        log::debug!("Search sources: call_id={}", args.call_id);

                                                                        if let Ok(json) = serde_json::to_string(&serde_json::json!({
                                                                            "_type": "sources",
                                                                            "call_id": args.call_id,
                                                                            "documents": args.documents
                                                                        })) {
                                                                            let _ = tx.send(Ok(format!("[MEILI_SOURCES:{}]", json))).await;
                                                                        }
                                                                    }
                                                                    Some(ParsedToolCall::Unknown { name, .. }) => {
                                                                        log::debug!("Unknown tool call: {}", name);
                                                                    }
                                                                    None => {
                                                                        log::warn!("Failed to parse tool call: {} - {}", func_name, func_args);
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    } else {
                                                        log::debug!("Received non-array tool_calls: {:?}", tool_calls);
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            // Try parsing as error
                                            if let Ok(error_response) = serde_json::from_str::<MeilisearchErrorResponse>(data) {
                                                let msg = &error_response.error.message;
                                                let error_code = error_response.error.code.as_deref().unwrap_or("");

                                                // MITIGATION: Detect and suppress LLM filter hallucination errors
                                                // The LLM sometimes generates SQL-like syntax or invalid operators
                                                // that Meilisearch doesn't understand. Instead of terminating,
                                                // we continue processing to let the LLM retry or respond without search.
                                                let is_filter_hallucination =
                                                    // Error code check
                                                    error_code == "invalid_search_filter" ||
                                                    // Pattern: Generic filter parse errors
                                                    msg.contains("Was expecting a value") ||
                                                    msg.contains("Was expecting an operation") ||
                                                    msg.contains("unexpected characters at the end of the filter") ||
                                                    // Pattern: Regex-like operators (=~, !~)
                                                    msg.contains("=~") ||
                                                    msg.contains("!~") ||
                                                    // Pattern: SQL-like operators the LLM hallucinates
                                                    msg.contains("'NULL'") ||
                                                    msg.contains("'IS NOT NULL'") ||
                                                    msg.contains("'IS EMPTY'") ||
                                                    msg.contains("'CONTAINS'") ||
                                                    msg.contains("'NOT CONTAINS'") ||
                                                    msg.contains("'STARTS WITH'") ||
                                                    msg.contains("'NOT STARTS WITH'") ||
                                                    msg.contains("'LIKE'") ||
                                                    msg.contains("'ILIKE'") ||
                                                    // Pattern: Geo filter errors
                                                    msg.contains("'_geoRadius'") ||
                                                    msg.contains("'_geoBoundingBox'") ||
                                                    msg.contains("'_geoPolygon'") ||
                                                    // Pattern: Attribute not filterable (when filterable_attributes disabled)
                                                    msg.contains("is not filterable") ||
                                                    // Pattern: Invalid filter syntax variations
                                                    (msg.contains("filter") && msg.contains("invalid"));

                                                if is_filter_hallucination {
                                                    log::warn!("Suppressed LLM filter hallucination error (continuing stream): {}", msg);
                                                    // Continue processing - don't terminate the stream
                                                    // The LLM will either retry the search or respond without RAG context
                                                    continue;
                                                }

                                                log::error!("Meilisearch API error: {}", msg);
                                                let _ = tx.send(Err(msg.clone())).await;
                                                return;
                                            }

                                            log::warn!("Failed to parse chunk: {} Data: {}", e, data);
                                            // Make id optional in struct if this often fails
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Stream error: {}", e);
                        let _ = tx.send(Err(e.to_string())).await;
                        return;
                    }
                }
            }

            // Handle any remaining buffer content (e.g. if stream ended without double newline)
            if !buffer.is_empty() {
                log::debug!("Stream ended with data in buffer: {}", buffer);
                if buffer.trim() == "data: [DONE]" {
                     let _ = tx.send(Ok("[DONE]".to_string())).await;
                }
            }
            log::info!("SSE stream ended");
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
            tools: Some(get_meilisearch_chat_tools()),
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
            ProviderConfig::ClaudeGate { model, max_tokens, .. } => ChatProviderConfig::ClaudeGate {
                model: model.clone(),
                max_tokens: Some(*max_tokens),
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
            ProviderConfig::ClaudeGate { model, max_tokens, .. } => ChatProviderConfig::ClaudeGate {
                model: model.clone(),
                max_tokens: Some(*max_tokens),
            },
            // Handle GeminiCLI as generic or unsupported for now if no direct map
            ProviderConfig::GeminiCli { .. } => return Err("Gemini CLI not supported for Meilisearch chat yet".to_string()),

            // Meilisearch provider is for using Meilisearch as a provider, creating a loop if we configure it here
            ProviderConfig::Meilisearch { .. } => return Err("Recursive Meilisearch configuration".to_string()),
        };

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
