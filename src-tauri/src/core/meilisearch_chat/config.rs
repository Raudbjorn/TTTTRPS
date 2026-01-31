//! Configuration types for Meilisearch Chat
//!
//! Contains LLM source configuration, workspace settings, and provider mapping logic.

use serde::{Deserialize, Serialize};

use crate::core::llm::model_selector::model_selector;
use crate::core::llm::providers::ProviderConfig;

use super::prompts::{
    DEFAULT_DM_SYSTEM_PROMPT, DEFAULT_SEARCH_DESCRIPTION, DEFAULT_SEARCH_INDEX_PARAM,
    DEFAULT_SEARCH_Q_PARAM, GROK_API_BASE_URL, GROK_DEFAULT_MODEL,
};

// ============================================================================
// LLM Source Types
// ============================================================================

/// LLM provider source for Meilisearch Chat
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ChatLLMSource {
    #[default]
    OpenAi,
    AzureOpenAi,
    Mistral,
    Google,
    VLlm,
}

// ============================================================================
// Prompt Configuration
// ============================================================================

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

// ============================================================================
// Workspace Settings
// ============================================================================

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

impl ChatWorkspaceSettings {
    /// Create settings for a native provider (direct API access)
    fn native(source: ChatLLMSource, api_key: String, base_url: Option<String>) -> Self {
        Self {
            source,
            api_key: Some(api_key),
            base_url,
            prompts: Some(ChatPrompts {
                system: Some(DEFAULT_DM_SYSTEM_PROMPT.to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    /// Create settings for a provider that routes through the proxy
    fn via_proxy(proxy_url: &str) -> Self {
        Self {
            source: ChatLLMSource::VLlm,
            api_key: None,
            base_url: Some(format!("{}/v1", proxy_url)),
            prompts: Some(ChatPrompts {
                system: Some(DEFAULT_DM_SYSTEM_PROMPT.to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    /// Create settings for Azure OpenAI
    fn azure(api_key: String, base_url: String, deployment_id: String, api_version: String) -> Self {
        Self {
            source: ChatLLMSource::AzureOpenAi,
            api_key: Some(api_key),
            deployment_id: Some(deployment_id),
            api_version: Some(api_version),
            base_url: Some(base_url),
            prompts: Some(ChatPrompts {
                system: Some(DEFAULT_DM_SYSTEM_PROMPT.to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }
    }
}

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
    /// Google (API key-based)
    Google {
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
    /// Claude OAuth (via proxy, no API key needed - uses OAuth tokens)
    ClaudeGate {
        model: String,
        #[serde(default)]
        max_tokens: Option<u32>,
    },
    /// Gemini OAuth (via proxy, no API key needed - uses OAuth tokens via gemini_gate)
    GeminiGate {
        model: String,
        #[serde(default)]
        max_tokens: Option<u32>,
    },
    /// Copilot (via proxy, no API key needed - uses Device Code OAuth tokens)
    CopilotGate {
        model: String,
        #[serde(default)]
        max_tokens: Option<u32>,
    },
}

impl ChatProviderConfig {
    /// Get the provider ID for proxy routing
    pub fn provider_id(&self) -> &'static str {
        match self {
            Self::OpenAI { .. } => "openai",
            Self::Claude { .. } | Self::ClaudeGate { .. } => "claude",
            Self::Mistral { .. } => "mistral",
            Self::Ollama { .. } => "ollama",
            Self::Google { .. } => "google",
            Self::OpenRouter { .. } => "openrouter",
            Self::AzureOpenAI { .. } => "azure",
            Self::Groq { .. } => "groq",
            Self::Together { .. } => "together",
            Self::Cohere { .. } => "cohere",
            Self::DeepSeek { .. } => "deepseek",
            Self::Grok { .. } => "grok",
            Self::GeminiGate { .. } => "gemini",
            Self::CopilotGate { .. } => "copilot",
        }
    }

    /// Check if this provider requires the proxy (vs native Meilisearch support)
    pub fn requires_proxy(&self) -> bool {
        !matches!(
            self,
            Self::OpenAI { .. }
                | Self::Mistral { .. }
                | Self::AzureOpenAI { .. }
                | Self::Google { .. }  // Native Meilisearch support
                | Self::Grok { .. }  // OpenAI-compatible
        )
    }

    /// Get the model identifier for proxy routing (format: provider:model)
    pub fn proxy_model_id(&self) -> String {
        let provider = self.provider_id();

        match self {
            // Providers with dynamic model selection
            Self::Claude { model, .. } => {
                let m = model.clone().unwrap_or_else(|| model_selector().select_model_sync());
                format!("claude:{}", m)
            }
            Self::ClaudeGate { model, .. } => format!("claude:{}", model),
            Self::GeminiGate { model, .. } => format!("gemini:{}", model),
            Self::CopilotGate { model, .. } => format!("copilot:{}", model),

            // Providers with explicit or default model
            Self::OpenAI { model, .. } => {
                format!("{}:{}", provider, model.as_deref().unwrap_or("gpt-4o-mini"))
            }
            Self::Mistral { model, .. } => {
                format!("{}:{}", provider, model.as_deref().unwrap_or("mistral-large-latest"))
            }
            Self::Google { model, .. } => {
                format!("{}:{}", provider, model.as_deref().unwrap_or("gemini-2.0-flash"))
            }
            Self::Grok { model, .. } => {
                format!("{}:{}", provider, model.as_deref().unwrap_or(GROK_DEFAULT_MODEL))
            }

            // Providers with required model field
            Self::Ollama { model, .. }
            | Self::OpenRouter { model, .. }
            | Self::Groq { model, .. }
            | Self::Together { model, .. }
            | Self::Cohere { model, .. }
            | Self::DeepSeek { model, .. } => {
                format!("{}:{}", provider, model)
            }

            // Azure uses deployment ID, not model name
            Self::AzureOpenAI { .. } => format!("{}:azure-deployment", provider),
        }
    }

    /// Convert to Meilisearch ChatWorkspaceSettings
    pub fn to_meilisearch_settings(&self, proxy_url: &str) -> ChatWorkspaceSettings {
        match self {
            // Native providers with direct API access
            Self::OpenAI { api_key, .. } => {
                ChatWorkspaceSettings::native(ChatLLMSource::OpenAi, api_key.clone(), None)
            }
            Self::Mistral { api_key, .. } => {
                ChatWorkspaceSettings::native(ChatLLMSource::Mistral, api_key.clone(), None)
            }
            Self::Google { api_key, .. } => {
                ChatWorkspaceSettings::native(ChatLLMSource::Google, api_key.clone(), None)
            }

            // Azure has its own configuration pattern
            Self::AzureOpenAI { api_key, base_url, deployment_id, api_version } => {
                ChatWorkspaceSettings::azure(
                    api_key.clone(),
                    base_url.clone(),
                    deployment_id.clone(),
                    api_version.clone(),
                )
            }

            // Grok uses VLlm source with xAI base URL (OpenAI-compatible)
            Self::Grok { api_key, .. } => {
                ChatWorkspaceSettings::native(
                    ChatLLMSource::VLlm,
                    api_key.clone(),
                    Some(GROK_API_BASE_URL.to_string()),
                )
            }

            // Ollama uses VLlm source with local base URL
            Self::Ollama { host, .. } => {
                let base_url = format!("{}/v1", host.trim_end_matches('/'));
                ChatWorkspaceSettings {
                    source: ChatLLMSource::VLlm,
                    api_key: Some("ollama".to_string()), // Placeholder key required by Meilisearch
                    base_url: Some(base_url),
                    prompts: Some(ChatPrompts {
                        system: Some(DEFAULT_DM_SYSTEM_PROMPT.to_string()),
                        ..Default::default()
                    }),
                    ..Default::default()
                }
            }

            // All other providers route through the proxy
            Self::Claude { .. }
            | Self::OpenRouter { .. }
            | Self::Groq { .. }
            | Self::Together { .. }
            | Self::Cohere { .. }
            | Self::DeepSeek { .. }
            | Self::ClaudeGate { .. }
            | Self::GeminiGate { .. }
            | Self::CopilotGate { .. } => ChatWorkspaceSettings::via_proxy(proxy_url),
        }
    }

    /// Convert to the project's ProviderConfig for proxy registration
    pub fn to_provider_config(&self) -> ProviderConfig {
        match self {
            Self::OpenAI { api_key, model, organization_id } => ProviderConfig::OpenAI {
                api_key: api_key.clone(),
                model: model.as_deref().unwrap_or("gpt-4o-mini").to_string(),
                max_tokens: 4096,
                organization_id: organization_id.clone(),
                base_url: None,
            },

            Self::Claude { model, max_tokens, .. } => ProviderConfig::Claude {
                storage_backend: "auto".to_string(),
                model: model.as_deref().unwrap_or("claude-sonnet-4-20250514").to_string(),
                max_tokens: max_tokens.unwrap_or(4096),
            },

            Self::Mistral { api_key, model } => ProviderConfig::Mistral {
                api_key: api_key.clone(),
                model: model.as_deref().unwrap_or("mistral-large-latest").to_string(),
            },

            Self::Ollama { host, model } => ProviderConfig::Ollama {
                host: host.clone(),
                model: model.clone(),
            },

            Self::Google { api_key, model } => ProviderConfig::Google {
                api_key: api_key.clone(),
                model: model.as_deref().unwrap_or("gemini-2.0-flash").to_string(),
            },

            Self::OpenRouter { api_key, model } => ProviderConfig::OpenRouter {
                api_key: api_key.clone(),
                model: model.clone(),
            },

            Self::AzureOpenAI { api_key, base_url, .. } => ProviderConfig::OpenAI {
                api_key: api_key.clone(),
                model: "azure".to_string(),
                max_tokens: 4096,
                organization_id: None,
                base_url: Some(base_url.clone()),
            },

            Self::Groq { api_key, model } => ProviderConfig::Groq {
                api_key: api_key.clone(),
                model: model.clone(),
            },

            Self::Together { api_key, model } => ProviderConfig::Together {
                api_key: api_key.clone(),
                model: model.clone(),
            },

            Self::Cohere { api_key, model } => ProviderConfig::Cohere {
                api_key: api_key.clone(),
                model: model.clone(),
            },

            Self::DeepSeek { api_key, model } => ProviderConfig::DeepSeek {
                api_key: api_key.clone(),
                model: model.clone(),
            },

            // Grok uses OpenAI-compatible API
            Self::Grok { api_key, model } => ProviderConfig::OpenAI {
                api_key: api_key.clone(),
                model: model.as_deref().unwrap_or(GROK_DEFAULT_MODEL).to_string(),
                max_tokens: 4096,
                organization_id: None,
                base_url: Some(GROK_API_BASE_URL.to_string()),
            },

            // OAuth-based providers (no API key, use gate services)
            Self::ClaudeGate { model, max_tokens } => ProviderConfig::Claude {
                storage_backend: "auto".to_string(),
                model: model.clone(),
                max_tokens: max_tokens.unwrap_or(8192),
            },

            Self::GeminiGate { model, max_tokens } => ProviderConfig::Gemini {
                storage_backend: "auto".to_string(),
                model: model.clone(),
                max_tokens: max_tokens.unwrap_or(8192),
            },

            Self::CopilotGate { model, max_tokens } => ProviderConfig::Copilot {
                storage_backend: "auto".to_string(),
                model: model.clone(),
                max_tokens: max_tokens.unwrap_or(8192),
            },
        }
    }
}

/// Convert from ProviderConfig to ChatProviderConfig
impl TryFrom<&ProviderConfig> for ChatProviderConfig {
    type Error = &'static str;

    fn try_from(config: &ProviderConfig) -> Result<Self, Self::Error> {
        match config {
            ProviderConfig::OpenAI { api_key, model, organization_id, .. } => {
                Ok(ChatProviderConfig::OpenAI {
                    api_key: api_key.clone(),
                    model: Some(model.clone()),
                    organization_id: organization_id.clone(),
                })
            }
            ProviderConfig::Mistral { api_key, model } => Ok(ChatProviderConfig::Mistral {
                api_key: api_key.clone(),
                model: Some(model.clone()),
            }),
            ProviderConfig::Ollama { host, model } => Ok(ChatProviderConfig::Ollama {
                host: host.clone(),
                model: model.clone(),
            }),
            ProviderConfig::Google { api_key, model } => Ok(ChatProviderConfig::Google {
                api_key: api_key.clone(),
                model: Some(model.clone()),
            }),
            ProviderConfig::OpenRouter { api_key, model } => Ok(ChatProviderConfig::OpenRouter {
                api_key: api_key.clone(),
                model: model.clone(),
            }),
            ProviderConfig::Groq { api_key, model } => Ok(ChatProviderConfig::Groq {
                api_key: api_key.clone(),
                model: model.clone(),
            }),
            ProviderConfig::Together { api_key, model } => Ok(ChatProviderConfig::Together {
                api_key: api_key.clone(),
                model: model.clone(),
            }),
            ProviderConfig::Cohere { api_key, model } => Ok(ChatProviderConfig::Cohere {
                api_key: api_key.clone(),
                model: model.clone(),
            }),
            ProviderConfig::DeepSeek { api_key, model } => Ok(ChatProviderConfig::DeepSeek {
                api_key: api_key.clone(),
                model: model.clone(),
            }),
            ProviderConfig::Claude { model, max_tokens, .. } => Ok(ChatProviderConfig::ClaudeGate {
                model: model.clone(),
                max_tokens: Some(*max_tokens),
            }),
            ProviderConfig::Gemini { model, max_tokens, .. } => Ok(ChatProviderConfig::GeminiGate {
                model: model.clone(),
                max_tokens: Some(*max_tokens),
            }),
            ProviderConfig::Copilot { model, max_tokens, .. } => Ok(ChatProviderConfig::CopilotGate {
                model: model.clone(),
                max_tokens: Some(*max_tokens),
            }),
            ProviderConfig::Meilisearch { .. } => Err("Recursive Meilisearch configuration"),
        }
    }
}
