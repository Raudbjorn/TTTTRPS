//! LLM Provider enumeration for RAG configuration

use serde::{Deserialize, Serialize};

/// Supported LLM providers for RAG queries
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
    /// Anthropic (Claude models)
    Anthropic,
    /// OpenAI (GPT models)
    OpenAi,
    /// Mistral AI
    Mistral,
    /// vLLM (self-hosted)
    VLlm {
        /// Base URL for vLLM API
        base_url: String,
    },
    /// Ollama (local models)
    Ollama {
        /// Base URL for Ollama API (default: http://localhost:11434)
        #[serde(default = "default_ollama_url")]
        base_url: String,
    },
    /// Azure OpenAI
    Azure {
        /// Azure endpoint URL
        base_url: String,
        /// Deployment ID
        deployment_id: String,
        /// API version
        api_version: String,
    },
}

fn default_ollama_url() -> String {
    "http://localhost:11434".to_string()
}

impl Default for LlmProvider {
    fn default() -> Self {
        Self::Ollama {
            base_url: default_ollama_url(),
        }
    }
}

impl LlmProvider {
    /// Get the provider name as a string
    pub fn name(&self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAi => "openai",
            Self::Mistral => "mistral",
            Self::VLlm { .. } => "vllm",
            Self::Ollama { .. } => "ollama",
            Self::Azure { .. } => "azure",
        }
    }

    /// Check if this is a local provider (no API key required)
    pub fn is_local(&self) -> bool {
        matches!(self, Self::Ollama { .. } | Self::VLlm { .. })
    }

    /// Get the base URL if applicable
    pub fn base_url(&self) -> Option<&str> {
        match self {
            Self::VLlm { base_url } => Some(base_url),
            Self::Ollama { base_url } => Some(base_url),
            Self::Azure { base_url, .. } => Some(base_url),
            _ => None,
        }
    }
}

/// Parse a provider string into an LlmProvider enum
pub fn parse_provider(provider: &str) -> Result<LlmProvider, String> {
    match provider.to_lowercase().as_str() {
        "anthropic" | "claude" => Ok(LlmProvider::Anthropic),
        "openai" | "gpt" => Ok(LlmProvider::OpenAi),
        "mistral" => Ok(LlmProvider::Mistral),
        "ollama" => Ok(LlmProvider::Ollama {
            base_url: default_ollama_url(),
        }),
        other => Err(format!("Unknown LLM provider: {}", other)),
    }
}
