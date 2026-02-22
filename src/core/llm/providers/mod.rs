//! LLM Provider Implementations
//!
//! This module contains concrete implementations of the `LLMProvider` trait
//! for all supported LLM providers.

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
mod meilisearch;

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
pub use meilisearch::MeilisearchProvider;

use super::router::LLMProvider;
use std::sync::Arc;

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
        storage_backend: String,  // Storage backend: "file", "keyring", "memory", "auto"
        model: String,            // Model to use (e.g., "gemini-2.0-flash")
        max_tokens: u32,          // Max tokens for responses (default 8192)
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
        storage_backend: String,  // Storage backend: "file", "keyring", "memory", "auto"
        model: String,            // Model to use (e.g., "claude-sonnet-4-20250514")
        max_tokens: u32,          // Max tokens for responses (default 8192)
    },
    /// Copilot (Device Code OAuth-based, no API key needed)
    Copilot {
        storage_backend: String,  // Storage backend: "file", "keyring", "memory", "auto"
        model: String,            // Model to use (e.g., "gpt-4o")
        max_tokens: u32,          // Max tokens for responses (default 8192)
    },
    Meilisearch {
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
                // Attempt to create the provider; fall back to memory storage on failure
                // In practice, the caller should validate configuration beforehand
                match ClaudeProvider::from_storage_name(storage_backend, model.clone(), *max_tokens) {
                    Ok(provider) => Arc::new(provider),
                    Err(e) => {
                        // Fall back to memory storage on error to avoid panicking
                        tracing::warn!("Failed to create Claude provider with {} storage: {}. Falling back to memory.", storage_backend, e);
                        Arc::new(ClaudeProvider::with_memory().expect("Memory storage should always work"))
                    }
                }
            }
            ProviderConfig::Gemini { storage_backend, model, max_tokens } => {
                // Attempt to create the OAuth-based Gemini provider; fall back to memory storage on failure
                match GeminiProvider::from_storage_name(storage_backend, model.clone(), *max_tokens) {
                    Ok(provider) => Arc::new(provider),
                    Err(e) => {
                        tracing::warn!("Failed to create Gemini provider with {} storage: {}. Falling back to memory.", storage_backend, e);
                        Arc::new(GeminiProvider::with_memory().expect("Memory storage should always work"))
                    }
                }
            }
            ProviderConfig::Copilot { storage_backend, model, max_tokens } => {
                // Attempt to create the Device Code OAuth-based Copilot provider; fall back to memory storage on failure
                match CopilotLLMProvider::from_storage_name(storage_backend, model.clone(), *max_tokens) {
                    Ok(provider) => Arc::new(provider),
                    Err(e) => {
                        tracing::warn!("Failed to create Copilot provider with {} storage: {}. Falling back to memory.", storage_backend, e);
                        Arc::new(CopilotLLMProvider::with_memory().expect("Memory storage should always work"))
                    }
                }
            }
            ProviderConfig::Meilisearch { host, api_key, workspace_id, model } => {
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
            ProviderConfig::Meilisearch { .. } => "meilisearch",
        }
    }

    /// Check if this provider requires the LLM Proxy for Meilisearch chat
    pub fn requires_proxy(&self) -> bool {
        match self {
            // Natively supported by Meilisearch
            ProviderConfig::OpenAI { .. } => false,
            ProviderConfig::Google { .. } => false, // Meilisearch supports Google/Gemini natively
            ProviderConfig::Mistral { .. } => false,
            ProviderConfig::Ollama { .. } => false, // Uses vLLM source which is supported

            // Others need proxy to look like OpenAI
            ProviderConfig::Claude { .. } => true,
            ProviderConfig::Gemini { .. } => true, // OAuth-based Gemini needs proxy
            ProviderConfig::Copilot { .. } => true, // Copilot uses OpenAI format but needs auth proxy

            // OpenAI-compatible but might need header tweaking or proxy for consistency
            ProviderConfig::OpenRouter { .. } => true,
            ProviderConfig::Groq { .. } => true,
            ProviderConfig::Together { .. } => true,
            ProviderConfig::Cohere { .. } => true,
            ProviderConfig::DeepSeek { .. } => true,

            // Meilisearch itself doesn't need proxy
            ProviderConfig::Meilisearch { .. } => false,
        }
    }

    /// Get the model name for this configuration
    pub fn model_name(&self) -> String {
        match self {
            ProviderConfig::Ollama { model, .. } => model.clone(),
            ProviderConfig::Claude { model, .. } => model.clone(),
            ProviderConfig::OpenAI { model, .. } => model.clone(),
            ProviderConfig::Google { model, .. } => model.clone(),
            ProviderConfig::Gemini { model, .. } => model.clone(),
            ProviderConfig::Copilot { model, .. } => model.clone(),
            ProviderConfig::OpenRouter { model, .. } => model.clone(),
            ProviderConfig::Mistral { model, .. } => model.clone(),
            ProviderConfig::Groq { model, .. } => model.clone(),
            ProviderConfig::Together { model, .. } => model.clone(),
            ProviderConfig::Cohere { model, .. } => model.clone(),
            ProviderConfig::DeepSeek { model, .. } => model.clone(),
            ProviderConfig::Meilisearch { model, .. } => model.clone(),
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
        // Native providers don't need proxy
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
        // Non-native providers need proxy
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
}
