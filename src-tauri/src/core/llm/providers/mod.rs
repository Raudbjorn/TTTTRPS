//! LLM Provider Implementations
//!
//! This module contains concrete implementations of the `LLMProvider` trait
//! for all supported LLM providers.

mod ollama;
mod claude;
mod claude_code;
mod claude_desktop;
mod openai;
mod gemini;
mod openrouter;
mod mistral;
mod groq;
mod together;
mod cohere;
mod deepseek;

pub use ollama::OllamaProvider;
pub use claude::ClaudeProvider;
pub use claude_code::{ClaudeCodeProvider, ClaudeCodeStatus};
pub use claude_desktop::ClaudeDesktopProvider;
pub use openai::OpenAIProvider;
pub use gemini::GeminiProvider;
pub use openrouter::OpenRouterProvider;
pub use mistral::MistralProvider;
pub use groq::GroqProvider;
pub use together::TogetherProvider;
pub use cohere::CohereProvider;
pub use deepseek::DeepSeekProvider;

use super::router::LLMProvider;
use std::sync::Arc;

/// Configuration for creating providers
#[derive(Debug, Clone)]
pub enum ProviderConfig {
    Ollama {
        host: String,
        model: String,
    },
    Claude {
        api_key: String,
        model: String,
        max_tokens: u32,
    },
    OpenAI {
        api_key: String,
        model: String,
        max_tokens: u32,
        organization_id: Option<String>,
        base_url: Option<String>,
    },
    Gemini {
        api_key: String,
        model: String,
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
    /// Claude Desktop via CDP (no API key needed, uses existing Claude Desktop auth)
    ClaudeDesktop {
        port: u16,          // CDP port (default 9333)
        timeout_secs: u64,  // Response timeout (default 120s)
    },
    /// Claude Code via CLI (no API key needed, uses existing Claude Code auth)
    ClaudeCode {
        timeout_secs: u64,          // Response timeout (default 300s)
        model: Option<String>,      // Optional model override
        working_dir: Option<String>, // Optional working directory
    },
}

impl ProviderConfig {
    /// Create a provider from this configuration
    pub fn create_provider(&self) -> Arc<dyn LLMProvider> {
        match self {
            ProviderConfig::Ollama { host, model } => {
                Arc::new(OllamaProvider::new(host.clone(), model.clone()))
            }
            ProviderConfig::Claude { api_key, model, max_tokens } => {
                Arc::new(ClaudeProvider::new(api_key.clone(), model.clone(), *max_tokens))
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
            ProviderConfig::Gemini { api_key, model } => {
                Arc::new(GeminiProvider::new(api_key.clone(), model.clone()))
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
            ProviderConfig::ClaudeDesktop { port, timeout_secs } => {
                Arc::new(ClaudeDesktopProvider::with_config(*port, *timeout_secs))
            }
            ProviderConfig::ClaudeCode { timeout_secs, model, working_dir } => {
                Arc::new(ClaudeCodeProvider::with_config(*timeout_secs, model.clone(), working_dir.clone()))
            }
        }
    }

    /// Get the provider ID for this configuration
    pub fn provider_id(&self) -> &'static str {
        match self {
            ProviderConfig::Ollama { .. } => "ollama",
            ProviderConfig::Claude { .. } => "claude",
            ProviderConfig::OpenAI { .. } => "openai",
            ProviderConfig::Gemini { .. } => "gemini",
            ProviderConfig::OpenRouter { .. } => "openrouter",
            ProviderConfig::Mistral { .. } => "mistral",
            ProviderConfig::Groq { .. } => "groq",
            ProviderConfig::Together { .. } => "together",
            ProviderConfig::Cohere { .. } => "cohere",
            ProviderConfig::DeepSeek { .. } => "deepseek",
            ProviderConfig::ClaudeDesktop { .. } => "claude-desktop",
            ProviderConfig::ClaudeCode { .. } => "claude-code",
        }
    }
}
