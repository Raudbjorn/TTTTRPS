//! LLM Provider Implementations
//!
//! This module contains concrete implementations of the `LLMProvider` trait
//! for all supported LLM providers, plus the canonical provider metadata table.
//!
//! Adding a new provider requires:
//! 1. A new enum variant in `ProviderConfig`
//! 2. A new entry in `PROVIDERS`
//! 3. The provider implementation file

mod ollama;
mod claude;
mod openai;
mod google;
mod gemini;
mod copilot;
mod openrouter;
mod mistral;
mod groq;
mod together;
mod cohere;
mod deepseek;
mod search;

pub use ollama::OllamaProvider;
pub use claude::{ClaudeProvider, ClaudeStatus, StorageBackend};
pub use openai::OpenAIProvider;
pub use google::GoogleProvider;
pub use gemini::{GeminiProvider, GeminiStatus, GeminiStorageBackend};
pub use copilot::{CopilotLLMProvider, CopilotStatus, CopilotStorageBackend};
pub use openrouter::OpenRouterProvider;
pub use mistral::MistralProvider;
pub use groq::GroqProvider;
pub use together::TogetherProvider;
pub use cohere::CohereProvider;
pub use deepseek::DeepSeekProvider;
pub use search::MeilisearchProvider;

use super::router::LLMProvider;
use std::sync::Arc;

// ── Auth method ─────────────────────────────────────────────────────────────

/// How a provider authenticates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AuthMethod {
    /// Standard API key input field.
    ApiKey,
    /// Host URL only (Ollama).
    HostOnly,
    /// Browser-based OAuth PKCE: open URL, user pastes authorization code.
    OAuthPkce,
    /// GitHub Device Code: display code, poll in background.
    DeviceCode,
}

// ── Provider metadata ───────────────────────────────────────────────────────

/// Static metadata for a known provider (display name, auth method, defaults).
#[derive(Clone, Debug)]
pub struct ProviderMeta {
    pub id: &'static str,
    pub display_name: &'static str,
    pub auth_method: AuthMethod,
    pub default_model: &'static str,
    pub key_placeholder: &'static str,
}

impl ProviderMeta {
    pub fn needs_api_key(&self) -> bool {
        self.auth_method == AuthMethod::ApiKey
    }

    pub fn needs_host(&self) -> bool {
        self.auth_method == AuthMethod::HostOnly
    }

    /// Short tag shown in the provider selector.
    pub fn auth_tag(&self) -> &'static str {
        match self.auth_method {
            AuthMethod::ApiKey => "key",
            AuthMethod::HostOnly => "local",
            AuthMethod::OAuthPkce => "OAuth",
            AuthMethod::DeviceCode => "device",
        }
    }
}

/// Canonical table of all known providers. Single source of truth.
pub const PROVIDERS: &[ProviderMeta] = &[
    ProviderMeta {
        id: "ollama",
        display_name: "Ollama (Local)",
        auth_method: AuthMethod::HostOnly,
        default_model: "llama3.2",
        key_placeholder: "",
    },
    ProviderMeta {
        id: "openai",
        display_name: "OpenAI",
        auth_method: AuthMethod::ApiKey,
        default_model: "gpt-4o",
        key_placeholder: "sk-...",
    },
    ProviderMeta {
        id: "anthropic",
        display_name: "Anthropic (API Key)",
        auth_method: AuthMethod::ApiKey,
        default_model: "claude-sonnet-4-20250514",
        key_placeholder: "sk-ant-...",
    },
    ProviderMeta {
        id: "google",
        display_name: "Google AI (API Key)",
        auth_method: AuthMethod::ApiKey,
        default_model: "gemini-2.0-flash",
        key_placeholder: "AIza...",
    },
    ProviderMeta {
        id: "claude",
        display_name: "Claude (OAuth)",
        auth_method: AuthMethod::OAuthPkce,
        default_model: "claude-sonnet-4-20250514",
        key_placeholder: "",
    },
    ProviderMeta {
        id: "gemini",
        display_name: "Gemini (OAuth)",
        auth_method: AuthMethod::OAuthPkce,
        default_model: "gemini-2.0-flash",
        key_placeholder: "",
    },
    ProviderMeta {
        id: "copilot",
        display_name: "GitHub Copilot",
        auth_method: AuthMethod::DeviceCode,
        default_model: "gpt-4o",
        key_placeholder: "",
    },
    ProviderMeta {
        id: "openrouter",
        display_name: "OpenRouter",
        auth_method: AuthMethod::ApiKey,
        default_model: "anthropic/claude-3.5-sonnet",
        key_placeholder: "sk-or-...",
    },
    ProviderMeta {
        id: "mistral",
        display_name: "Mistral",
        auth_method: AuthMethod::ApiKey,
        default_model: "mistral-large-latest",
        key_placeholder: "",
    },
    ProviderMeta {
        id: "groq",
        display_name: "Groq",
        auth_method: AuthMethod::ApiKey,
        default_model: "llama-3.3-70b-versatile",
        key_placeholder: "gsk_...",
    },
    ProviderMeta {
        id: "together",
        display_name: "Together AI",
        auth_method: AuthMethod::ApiKey,
        default_model: "meta-llama/Meta-Llama-3.1-70B",
        key_placeholder: "",
    },
    ProviderMeta {
        id: "cohere",
        display_name: "Cohere",
        auth_method: AuthMethod::ApiKey,
        default_model: "command-r-plus",
        key_placeholder: "",
    },
    ProviderMeta {
        id: "deepseek",
        display_name: "DeepSeek",
        auth_method: AuthMethod::ApiKey,
        default_model: "deepseek-chat",
        key_placeholder: "sk-...",
    },
];

/// Look up a provider's metadata by ID.
pub fn find_provider_meta(id: &str) -> Option<&'static ProviderMeta> {
    PROVIDERS.iter().find(|p| p.id == id)
}

// ── ProviderConfig ──────────────────────────────────────────────────────────

/// Configuration for creating providers
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ProviderConfig {
    Ollama {
        host: String,
        model: String,
    },
    OpenAI {
        api_key: String,
        model: String,
        max_tokens: u32,
        organization_id: Option<String>,
        base_url: Option<String>,
    },
    /// Google Gemini (API key-based)
    Google {
        api_key: String,
        model: String,
    },
    /// Gemini (OAuth-based via Cloud Code API, no API key needed)
    Gemini {
        storage_backend: String,
        model: String,
        max_tokens: u32,
    },
    OpenRouter {
        api_key: String,
        model: String,
    },
    Mistral {
        api_key: String,
        model: String,
    },
    Groq {
        api_key: String,
        model: String,
    },
    Together {
        api_key: String,
        model: String,
    },
    Cohere {
        api_key: String,
        model: String,
    },
    DeepSeek {
        api_key: String,
        model: String,
    },
    /// Claude (OAuth-based, no API key needed)
    Claude {
        storage_backend: String,
        model: String,
        max_tokens: u32,
    },
    /// Copilot (Device Code OAuth-based, no API key needed)
    Copilot {
        storage_backend: String,
        model: String,
        max_tokens: u32,
    },
    Search {
        host: String,
        api_key: Option<String>,
        workspace_id: String,
        model: String,
    },
}

impl ProviderConfig {
    /// Create a provider from this configuration
    pub fn create_provider(&self) -> Arc<dyn LLMProvider> {
        match self {
            ProviderConfig::Ollama { host, model } => {
                Arc::new(OllamaProvider::new(host.clone(), model.clone()))
            }
            ProviderConfig::OpenAI { api_key, model, max_tokens, organization_id, base_url } => {
                Arc::new(OpenAIProvider::new(
                    api_key.clone(),
                    model.clone(),
                    *max_tokens,
                    organization_id.clone(),
                    base_url.clone(),
                ))
            }
            ProviderConfig::Google { api_key, model } => {
                Arc::new(GoogleProvider::new(api_key.clone(), model.clone()))
            }
            ProviderConfig::OpenRouter { api_key, model } => {
                Arc::new(OpenRouterProvider::new(api_key.clone(), model.clone()))
            }
            ProviderConfig::Mistral { api_key, model } => {
                Arc::new(MistralProvider::new(api_key.clone(), model.clone()))
            }
            ProviderConfig::Groq { api_key, model } => {
                Arc::new(GroqProvider::new(api_key.clone(), model.clone()))
            }
            ProviderConfig::Together { api_key, model } => {
                Arc::new(TogetherProvider::new(api_key.clone(), model.clone()))
            }
            ProviderConfig::Cohere { api_key, model } => {
                Arc::new(CohereProvider::new(api_key.clone(), model.clone()))
            }
            ProviderConfig::DeepSeek { api_key, model } => {
                Arc::new(DeepSeekProvider::new(api_key.clone(), model.clone()))
            }
            ProviderConfig::Claude { storage_backend, model, max_tokens } => {
                match ClaudeProvider::from_storage_name(storage_backend, model.clone(), *max_tokens) {
                    Ok(provider) => Arc::new(provider),
                    Err(e) => {
                        tracing::warn!("Failed to create Claude provider with {} storage: {}. Falling back to memory.", storage_backend, e);
                        Arc::new(ClaudeProvider::with_memory().expect("Memory storage should always work"))
                    }
                }
            }
            ProviderConfig::Gemini { storage_backend, model, max_tokens } => {
                match GeminiProvider::from_storage_name(storage_backend, model.clone(), *max_tokens) {
                    Ok(provider) => Arc::new(provider),
                    Err(e) => {
                        tracing::warn!("Failed to create Gemini provider with {} storage: {}. Falling back to memory.", storage_backend, e);
                        Arc::new(GeminiProvider::with_memory().expect("Memory storage should always work"))
                    }
                }
            }
            ProviderConfig::Copilot { storage_backend, model, max_tokens } => {
                match CopilotLLMProvider::from_storage_name(storage_backend, model.clone(), *max_tokens) {
                    Ok(provider) => Arc::new(provider),
                    Err(e) => {
                        tracing::warn!("Failed to create Copilot provider with {} storage: {}. Falling back to memory.", storage_backend, e);
                        Arc::new(CopilotLLMProvider::with_memory().expect("Memory storage should always work"))
                    }
                }
            }
            ProviderConfig::Search { host, api_key, workspace_id, model } => {
                Arc::new(MeilisearchProvider::new(host.clone(), api_key.clone(), workspace_id.clone(), model.clone()))
            }
        }
    }

    /// Get the provider ID for this configuration
    pub fn provider_id(&self) -> &'static str {
        match self {
            ProviderConfig::Ollama { .. } => "ollama",
            ProviderConfig::Claude { .. } => "claude",
            ProviderConfig::OpenAI { .. } => "openai",
            ProviderConfig::Google { .. } => "google",
            ProviderConfig::Gemini { .. } => "gemini",
            ProviderConfig::Copilot { .. } => "copilot",
            ProviderConfig::OpenRouter { .. } => "openrouter",
            ProviderConfig::Mistral { .. } => "mistral",
            ProviderConfig::Groq { .. } => "groq",
            ProviderConfig::Together { .. } => "together",
            ProviderConfig::Cohere { .. } => "cohere",
            ProviderConfig::DeepSeek { .. } => "deepseek",
            ProviderConfig::Search { .. } => "meilisearch",
        }
    }

    /// Check if this provider requires the LLM Proxy
    pub fn requires_proxy(&self) -> bool {
        match self {
            // Natively supported by Meilisearch
            ProviderConfig::OpenAI { .. } => false,
            ProviderConfig::Google { .. } => false,
            ProviderConfig::Mistral { .. } => false,
            ProviderConfig::Ollama { .. } => false,

            // Others need proxy to look like OpenAI
            ProviderConfig::Claude { .. } => true,
            ProviderConfig::Gemini { .. } => true,
            ProviderConfig::Copilot { .. } => true,

            // OpenAI-compatible but might need header tweaking or proxy
            ProviderConfig::OpenRouter { .. } => true,
            ProviderConfig::Groq { .. } => true,
            ProviderConfig::Together { .. } => true,
            ProviderConfig::Cohere { .. } => true,
            ProviderConfig::DeepSeek { .. } => true,

            // Meilisearch itself doesn't need proxy
            ProviderConfig::Search { .. } => false,
        }
    }

    /// Get the model name for this configuration
    pub fn model_name(&self) -> String {
        match self {
            ProviderConfig::Ollama { model, .. }
            | ProviderConfig::Claude { model, .. }
            | ProviderConfig::OpenAI { model, .. }
            | ProviderConfig::Google { model, .. }
            | ProviderConfig::Gemini { model, .. }
            | ProviderConfig::Copilot { model, .. }
            | ProviderConfig::OpenRouter { model, .. }
            | ProviderConfig::Mistral { model, .. }
            | ProviderConfig::Groq { model, .. }
            | ProviderConfig::Together { model, .. }
            | ProviderConfig::Cohere { model, .. }
            | ProviderConfig::DeepSeek { model, .. }
            | ProviderConfig::Search { model, .. } => model.clone(),
        }
    }

    /// Derive the auth method from the variant.
    pub fn auth_method(&self) -> AuthMethod {
        match self {
            ProviderConfig::Ollama { .. } => AuthMethod::HostOnly,
            ProviderConfig::Claude { .. } => AuthMethod::OAuthPkce,
            ProviderConfig::Gemini { .. } => AuthMethod::OAuthPkce,
            ProviderConfig::Copilot { .. } => AuthMethod::DeviceCode,
            ProviderConfig::OpenAI { .. }
            | ProviderConfig::Google { .. }
            | ProviderConfig::OpenRouter { .. }
            | ProviderConfig::Mistral { .. }
            | ProviderConfig::Groq { .. }
            | ProviderConfig::Together { .. }
            | ProviderConfig::Cohere { .. }
            | ProviderConfig::DeepSeek { .. }
            | ProviderConfig::Search { .. } => AuthMethod::ApiKey,
        }
    }

    /// Extract the API key if this variant carries one.
    pub fn api_key(&self) -> Option<&str> {
        match self {
            ProviderConfig::OpenAI { api_key, .. }
            | ProviderConfig::Google { api_key, .. }
            | ProviderConfig::OpenRouter { api_key, .. }
            | ProviderConfig::Mistral { api_key, .. }
            | ProviderConfig::Groq { api_key, .. }
            | ProviderConfig::Together { api_key, .. }
            | ProviderConfig::Cohere { api_key, .. }
            | ProviderConfig::DeepSeek { api_key, .. } => {
                if api_key.is_empty() { None } else { Some(api_key) }
            }
            ProviderConfig::Search { api_key, .. } => api_key.as_deref(),
            ProviderConfig::Ollama { .. }
            | ProviderConfig::Claude { .. }
            | ProviderConfig::Gemini { .. }
            | ProviderConfig::Copilot { .. } => None,
        }
    }

    /// Return a clone with the API key injected.
    /// No-op for OAuth/HostOnly variants.
    pub fn with_api_key(&self, key: &str) -> Self {
        match self {
            ProviderConfig::OpenAI { model, max_tokens, organization_id, base_url, .. } => {
                ProviderConfig::OpenAI {
                    api_key: key.to_string(),
                    model: model.clone(),
                    max_tokens: *max_tokens,
                    organization_id: organization_id.clone(),
                    base_url: base_url.clone(),
                }
            }
            ProviderConfig::Google { model, .. } => {
                ProviderConfig::Google { api_key: key.to_string(), model: model.clone() }
            }
            ProviderConfig::OpenRouter { model, .. } => {
                ProviderConfig::OpenRouter { api_key: key.to_string(), model: model.clone() }
            }
            ProviderConfig::Mistral { model, .. } => {
                ProviderConfig::Mistral { api_key: key.to_string(), model: model.clone() }
            }
            ProviderConfig::Groq { model, .. } => {
                ProviderConfig::Groq { api_key: key.to_string(), model: model.clone() }
            }
            ProviderConfig::Together { model, .. } => {
                ProviderConfig::Together { api_key: key.to_string(), model: model.clone() }
            }
            ProviderConfig::Cohere { model, .. } => {
                ProviderConfig::Cohere { api_key: key.to_string(), model: model.clone() }
            }
            ProviderConfig::DeepSeek { model, .. } => {
                ProviderConfig::DeepSeek { api_key: key.to_string(), model: model.clone() }
            }
            // OAuth, HostOnly, Meilisearch — no-op
            other => other.clone(),
        }
    }

    /// Return a clone safe for disk persistence (API key stripped).
    pub fn without_secret(&self) -> Self {
        match self {
            ProviderConfig::OpenAI { model, max_tokens, organization_id, base_url, .. } => {
                ProviderConfig::OpenAI {
                    api_key: String::new(),
                    model: model.clone(),
                    max_tokens: *max_tokens,
                    organization_id: organization_id.clone(),
                    base_url: base_url.clone(),
                }
            }
            ProviderConfig::Google { model, .. } => {
                ProviderConfig::Google { api_key: String::new(), model: model.clone() }
            }
            ProviderConfig::OpenRouter { model, .. } => {
                ProviderConfig::OpenRouter { api_key: String::new(), model: model.clone() }
            }
            ProviderConfig::Mistral { model, .. } => {
                ProviderConfig::Mistral { api_key: String::new(), model: model.clone() }
            }
            ProviderConfig::Groq { model, .. } => {
                ProviderConfig::Groq { api_key: String::new(), model: model.clone() }
            }
            ProviderConfig::Together { model, .. } => {
                ProviderConfig::Together { api_key: String::new(), model: model.clone() }
            }
            ProviderConfig::Cohere { model, .. } => {
                ProviderConfig::Cohere { api_key: String::new(), model: model.clone() }
            }
            ProviderConfig::DeepSeek { model, .. } => {
                ProviderConfig::DeepSeek { api_key: String::new(), model: model.clone() }
            }
            // OAuth, HostOnly, Meilisearch — already safe
            other => other.clone(),
        }
    }

    /// Build a `ProviderConfig` from parts (provider ID + credentials).
    ///
    /// This is the single id-to-variant mapping point, replacing the old
    /// `build_provider_config()` in settings.rs. Note: "anthropic" maps to
    /// `OpenAI` with Anthropic's base_url (API-key compatible endpoint).
    pub fn from_parts(provider_id: &str, api_key: &str, host: &str, model: &str) -> Self {
        match provider_id {
            "ollama" => ProviderConfig::Ollama {
                host: host.to_string(),
                model: model.to_string(),
            },
            "openai" => ProviderConfig::OpenAI {
                api_key: api_key.to_string(),
                model: model.to_string(),
                max_tokens: 4096,
                organization_id: None,
                base_url: None,
            },
            "anthropic" => ProviderConfig::OpenAI {
                api_key: api_key.to_string(),
                model: model.to_string(),
                max_tokens: 8192,
                organization_id: None,
                base_url: Some("https://api.anthropic.com/v1".to_string()),
            },
            "google" => ProviderConfig::Google {
                api_key: api_key.to_string(),
                model: model.to_string(),
            },
            "claude" => ProviderConfig::Claude {
                storage_backend: "auto".to_string(),
                model: model.to_string(),
                max_tokens: 8192,
            },
            "gemini" => ProviderConfig::Gemini {
                storage_backend: "auto".to_string(),
                model: model.to_string(),
                max_tokens: 8192,
            },
            "copilot" => ProviderConfig::Copilot {
                storage_backend: "auto".to_string(),
                model: model.to_string(),
                max_tokens: 8192,
            },
            "openrouter" => ProviderConfig::OpenRouter {
                api_key: api_key.to_string(),
                model: model.to_string(),
            },
            "mistral" => ProviderConfig::Mistral {
                api_key: api_key.to_string(),
                model: model.to_string(),
            },
            "groq" => ProviderConfig::Groq {
                api_key: api_key.to_string(),
                model: model.to_string(),
            },
            "together" => ProviderConfig::Together {
                api_key: api_key.to_string(),
                model: model.to_string(),
            },
            "cohere" => ProviderConfig::Cohere {
                api_key: api_key.to_string(),
                model: model.to_string(),
            },
            "deepseek" => ProviderConfig::DeepSeek {
                api_key: api_key.to_string(),
                model: model.to_string(),
            },
            _ => ProviderConfig::Ollama {
                host: host.to_string(),
                model: model.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_config_provider_id() {
        let ollama = ProviderConfig::Ollama {
            host: "http://localhost:11434".to_string(),
            model: "llama2".to_string(),
        };
        assert_eq!(ollama.provider_id(), "ollama");

        let claude = ProviderConfig::Claude {
            storage_backend: "memory".to_string(),
            model: "claude-3-sonnet".to_string(),
            max_tokens: 4096,
        };
        assert_eq!(claude.provider_id(), "claude");

        let openai = ProviderConfig::OpenAI {
            api_key: "test".to_string(),
            model: "gpt-4".to_string(),
            max_tokens: 4096,
            base_url: None,
            organization_id: None,
        };
        assert_eq!(openai.provider_id(), "openai");
    }

    #[test]
    fn test_provider_config_requires_proxy_native() {
        let openai = ProviderConfig::OpenAI {
            api_key: "test".to_string(),
            model: "gpt-4".to_string(),
            max_tokens: 4096,
            base_url: None,
            organization_id: None,
        };
        assert!(!openai.requires_proxy());

        let ollama = ProviderConfig::Ollama {
            host: "http://localhost:11434".to_string(),
            model: "llama2".to_string(),
        };
        assert!(!ollama.requires_proxy());

        let mistral = ProviderConfig::Mistral {
            api_key: "test".to_string(),
            model: "mistral-large".to_string(),
        };
        assert!(!mistral.requires_proxy());
    }

    #[test]
    fn test_provider_config_requires_proxy_non_native() {
        let claude = ProviderConfig::Claude {
            storage_backend: "memory".to_string(),
            model: "claude-3-sonnet".to_string(),
            max_tokens: 4096,
        };
        assert!(claude.requires_proxy());

        let groq = ProviderConfig::Groq {
            api_key: "test".to_string(),
            model: "llama2-70b".to_string(),
        };
        assert!(groq.requires_proxy());

        let openrouter = ProviderConfig::OpenRouter {
            api_key: "test".to_string(),
            model: "anthropic/claude-3".to_string(),
        };
        assert!(openrouter.requires_proxy());
    }

    // ── AuthMethod / ProviderMeta tests ─────────────────────────────

    #[test]
    fn test_find_provider_meta() {
        assert!(find_provider_meta("openai").is_some());
        assert!(find_provider_meta("claude").is_some());
        assert!(find_provider_meta("nonexistent").is_none());
    }

    #[test]
    fn test_provider_meta_needs_api_key() {
        let openai = find_provider_meta("openai").unwrap();
        assert!(openai.needs_api_key());

        let ollama = find_provider_meta("ollama").unwrap();
        assert!(!ollama.needs_api_key());

        let claude = find_provider_meta("claude").unwrap();
        assert!(!claude.needs_api_key());
    }

    #[test]
    fn test_provider_meta_auth_tag() {
        assert_eq!(find_provider_meta("openai").unwrap().auth_tag(), "key");
        assert_eq!(find_provider_meta("ollama").unwrap().auth_tag(), "local");
        assert_eq!(find_provider_meta("claude").unwrap().auth_tag(), "OAuth");
        assert_eq!(find_provider_meta("copilot").unwrap().auth_tag(), "device");
    }

    // ── with_api_key / without_secret / api_key tests ───────────────

    #[test]
    fn test_with_api_key_injects_key() {
        let config = ProviderConfig::OpenAI {
            api_key: String::new(),
            model: "gpt-4o".to_string(),
            max_tokens: 4096,
            organization_id: None,
            base_url: None,
        };
        let injected = config.with_api_key("sk-test-123");
        assert_eq!(injected.api_key(), Some("sk-test-123"));
        assert_eq!(injected.model_name(), "gpt-4o");
    }

    #[test]
    fn test_with_api_key_noop_for_oauth() {
        let config = ProviderConfig::Claude {
            storage_backend: "auto".to_string(),
            model: "claude-3-sonnet".to_string(),
            max_tokens: 8192,
        };
        let result = config.with_api_key("sk-should-be-ignored");
        assert!(result.api_key().is_none());
        assert_eq!(result.provider_id(), "claude");
    }

    #[test]
    fn test_with_api_key_noop_for_host_only() {
        let config = ProviderConfig::Ollama {
            host: "http://localhost:11434".to_string(),
            model: "llama3".to_string(),
        };
        let result = config.with_api_key("sk-should-be-ignored");
        assert!(result.api_key().is_none());
        assert_eq!(result.provider_id(), "ollama");
    }

    #[test]
    fn test_without_secret_strips_key() {
        let config = ProviderConfig::OpenAI {
            api_key: "sk-secret".to_string(),
            model: "gpt-4o".to_string(),
            max_tokens: 4096,
            organization_id: None,
            base_url: None,
        };
        let stripped = config.without_secret();
        assert!(stripped.api_key().is_none());
        assert_eq!(stripped.model_name(), "gpt-4o");
    }

    #[test]
    fn test_without_secret_preserves_oauth() {
        let config = ProviderConfig::Claude {
            storage_backend: "auto".to_string(),
            model: "claude-3-sonnet".to_string(),
            max_tokens: 8192,
        };
        let stripped = config.without_secret();
        assert_eq!(stripped.provider_id(), "claude");
        assert_eq!(stripped.model_name(), "claude-3-sonnet");
    }

    #[test]
    fn test_api_key_returns_none_for_empty() {
        let config = ProviderConfig::OpenAI {
            api_key: String::new(),
            model: "gpt-4o".to_string(),
            max_tokens: 4096,
            organization_id: None,
            base_url: None,
        };
        assert!(config.api_key().is_none());
    }

    #[test]
    fn test_auth_method_derives_correctly() {
        let ollama = ProviderConfig::Ollama { host: String::new(), model: String::new() };
        assert_eq!(ollama.auth_method(), AuthMethod::HostOnly);

        let openai = ProviderConfig::OpenAI {
            api_key: String::new(), model: String::new(),
            max_tokens: 0, organization_id: None, base_url: None,
        };
        assert_eq!(openai.auth_method(), AuthMethod::ApiKey);

        let claude = ProviderConfig::Claude {
            storage_backend: String::new(), model: String::new(), max_tokens: 0,
        };
        assert_eq!(claude.auth_method(), AuthMethod::OAuthPkce);

        let copilot = ProviderConfig::Copilot {
            storage_backend: String::new(), model: String::new(), max_tokens: 0,
        };
        assert_eq!(copilot.auth_method(), AuthMethod::DeviceCode);
    }

    // ── from_parts tests (moved from settings.rs) ───────────────────

    #[test]
    fn test_from_parts_ollama() {
        let config = ProviderConfig::from_parts("ollama", "", "http://localhost:11434", "llama3.2");
        assert_eq!(config.provider_id(), "ollama");
        assert_eq!(config.model_name(), "llama3.2");
    }

    #[test]
    fn test_from_parts_openai() {
        let config = ProviderConfig::from_parts("openai", "sk-test", "", "gpt-4o");
        assert_eq!(config.provider_id(), "openai");
        assert_eq!(config.model_name(), "gpt-4o");
    }

    #[test]
    fn test_from_parts_anthropic() {
        let config = ProviderConfig::from_parts("anthropic", "sk-ant-test", "", "claude-sonnet-4-20250514");
        assert_eq!(config.provider_id(), "openai"); // maps to OpenAI with base_url
        assert_eq!(config.model_name(), "claude-sonnet-4-20250514");
        if let ProviderConfig::OpenAI { base_url, max_tokens, .. } = &config {
            assert_eq!(base_url.as_deref(), Some("https://api.anthropic.com/v1"));
            assert_eq!(*max_tokens, 8192);
        } else {
            panic!("Expected OpenAI config for anthropic");
        }
    }

    #[test]
    fn test_from_parts_google() {
        let config = ProviderConfig::from_parts("google", "AIzaTest", "", "gemini-2.0-flash");
        assert_eq!(config.provider_id(), "google");
        assert_eq!(config.model_name(), "gemini-2.0-flash");
    }

    #[test]
    fn test_from_parts_claude_oauth() {
        let config = ProviderConfig::from_parts("claude", "", "", "claude-sonnet-4-20250514");
        assert_eq!(config.provider_id(), "claude");
        assert_eq!(config.model_name(), "claude-sonnet-4-20250514");
        if let ProviderConfig::Claude { storage_backend, max_tokens, .. } = &config {
            assert_eq!(storage_backend, "auto");
            assert_eq!(*max_tokens, 8192);
        } else {
            panic!("Expected Claude config");
        }
    }

    #[test]
    fn test_from_parts_gemini_oauth() {
        let config = ProviderConfig::from_parts("gemini", "", "", "gemini-2.0-flash");
        assert_eq!(config.provider_id(), "gemini");
        if let ProviderConfig::Gemini { storage_backend, .. } = &config {
            assert_eq!(storage_backend, "auto");
        } else {
            panic!("Expected Gemini config");
        }
    }

    #[test]
    fn test_from_parts_copilot() {
        let config = ProviderConfig::from_parts("copilot", "", "", "gpt-4o");
        assert_eq!(config.provider_id(), "copilot");
        if let ProviderConfig::Copilot { storage_backend, .. } = &config {
            assert_eq!(storage_backend, "auto");
        } else {
            panic!("Expected Copilot config");
        }
    }

    #[test]
    fn test_from_parts_all_providers() {
        for p in PROVIDERS {
            let api_key = if p.needs_api_key() { "test-key" } else { "" };
            let host = if p.needs_host() {
                "http://localhost:11434"
            } else {
                ""
            };
            let config = ProviderConfig::from_parts(p.id, api_key, host, p.default_model);
            assert!(
                !config.model_name().is_empty(),
                "Model should be set for {}",
                p.id
            );
        }
    }

    #[test]
    fn test_with_api_key_all_api_key_providers() {
        for p in PROVIDERS.iter().filter(|p| p.needs_api_key()) {
            let config = ProviderConfig::from_parts(p.id, "", "", p.default_model);
            let injected = config.with_api_key("test-key-123");
            assert_eq!(
                injected.api_key(),
                Some("test-key-123"),
                "with_api_key should inject key for {}",
                p.id
            );
        }
    }

    #[test]
    fn test_without_secret_roundtrip() {
        for p in PROVIDERS.iter().filter(|p| p.needs_api_key()) {
            let config = ProviderConfig::from_parts(p.id, "secret-key", "", p.default_model);
            let stripped = config.without_secret();
            assert!(
                stripped.api_key().is_none(),
                "without_secret should strip key for {}",
                p.id
            );
            let restored = stripped.with_api_key("new-key");
            assert_eq!(
                restored.api_key(),
                Some("new-key"),
                "should be able to re-inject key for {}",
                p.id
            );
        }
    }
}
