//! RAG Configuration with TTRPG defaults
//!
//! This module provides the core configuration types for RAG (Retrieval-Augmented
//! Generation) in the TTRPG Assistant. It defines local Chat* types (previously
//! from meilisearch_lib) with sensible defaults for tabletop RPG content retrieval.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::provider::LlmProvider;
use super::templates::{
    CHUNK_TEMPLATE, FICTION_TEMPLATE, RULES_TEMPLATE, TTRPG_SYSTEM_PROMPT,
};

// ============================================================================
// Chat Configuration Types (local definitions)
// ============================================================================

/// LLM provider source for chat completions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChatSource {
    Anthropic,
    OpenAi,
    Mistral,
    VLlm,
    AzureOpenAi,
}

impl Default for ChatSource {
    fn default() -> Self {
        Self::OpenAi
    }
}

/// Prompt configuration for chat completions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatPrompts {
    /// System prompt for the LLM
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Description of the search tool
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_description: Option<String>,
    /// Description of the query parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_q_param: Option<String>,
    /// Description of the filter parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_filter_param: Option<String>,
    /// Description of the index selection parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_index_uid_param: Option<String>,
}

/// Search parameters for a chat index.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatSearchParams {
    /// Maximum number of documents to retrieve
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    /// Sort criteria
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<Vec<String>>,
    /// Matching strategy (e.g., "last", "all")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matching_strategy: Option<String>,
    /// Semantic ratio for hybrid search (0.0 = keyword only, 1.0 = semantic only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_ratio: Option<f32>,
    /// Embedder to use for semantic search
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedder: Option<String>,
}

/// Configuration for a single chat index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatIndexConfig {
    /// Description of the index for the LLM
    pub description: String,
    /// Liquid template for rendering documents
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    /// Maximum bytes per document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_bytes: Option<usize>,
    /// Search parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_params: Option<ChatSearchParams>,
}

/// Top-level chat configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatConfig {
    /// LLM provider source
    pub source: ChatSource,
    /// API key for the provider
    pub api_key: String,
    /// Base URL for the provider (required for vLLM, Azure, optional otherwise)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// Model identifier
    pub model: String,
    /// Organization ID (OpenAI)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_id: Option<String>,
    /// Project ID (OpenAI)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    /// API version (Azure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
    /// Deployment ID (Azure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_id: Option<String>,
    /// Prompt configuration
    pub prompts: ChatPrompts,
    /// Per-index configurations keyed by index UID
    pub index_configs: HashMap<String, ChatIndexConfig>,
}

// ============================================================================
// Constants
// ============================================================================

/// Default semantic ratio for rules content (favor semantic for conceptual queries)
pub const DEFAULT_SEMANTIC_RATIO_RULES: f32 = 0.7;

/// Default semantic ratio for fiction/lore content
pub const DEFAULT_SEMANTIC_RATIO_FICTION: f32 = 0.6;

/// Default semantic ratio for session notes
pub const DEFAULT_SEMANTIC_RATIO_NOTES: f32 = 0.5;

/// Default max bytes for rules documents (concise, focused)
pub const DEFAULT_MAX_BYTES_RULES: usize = 600;

/// Default max bytes for fiction documents (allow more narrative context)
pub const DEFAULT_MAX_BYTES_FICTION: usize = 1000;

/// Default max bytes for session notes
pub const DEFAULT_MAX_BYTES_NOTES: usize = 800;

/// Default max bytes for homebrew content
pub const DEFAULT_MAX_BYTES_HOMEBREW: usize = 700;

/// Default document limit for RAG retrieval
pub const DEFAULT_DOCUMENT_LIMIT: usize = 8;

// ============================================================================
// TTRPG Index Definitions
// ============================================================================

/// Known TTRPG content indexes with their configuration defaults.
///
/// Each variant maps to a specific Meilisearch index and carries metadata
/// about how to best search and present that content type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TtrpgIndex {
    /// Game rules, mechanics, and technical content (PHB, DMG, etc.)
    Rules,
    /// Fiction, lore, and narrative content (setting books, adventures)
    Fiction,
    /// Session notes and campaign logs
    SessionNotes,
    /// User-created homebrew content
    Homebrew,
}

impl TtrpgIndex {
    /// Get the Meilisearch index UID for this content type.
    pub fn index_uid(&self) -> &'static str {
        match self {
            Self::Rules => "ttrpg_rules",
            Self::Fiction => "ttrpg_fiction",
            Self::SessionNotes => "session_notes",
            Self::Homebrew => "homebrew",
        }
    }

    /// Get a human-readable description for the LLM.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Rules => "Official game rules, mechanics, character options, spells, items, and combat procedures. Use for questions about how the game works.",
            Self::Fiction => "Campaign setting lore, world history, faction descriptions, NPC backgrounds, and narrative content. Use for worldbuilding and storytelling questions.",
            Self::SessionNotes => "Session logs, player notes, campaign timeline events, and in-game happenings. Use for recalling past events and continuity.",
            Self::Homebrew => "Custom rules, house variants, user-created classes, monsters, and modifications. Use for campaign-specific customizations.",
        }
    }

    /// Get the default max_bytes for documents from this index.
    pub fn default_max_bytes(&self) -> usize {
        match self {
            Self::Rules => DEFAULT_MAX_BYTES_RULES,
            Self::Fiction => DEFAULT_MAX_BYTES_FICTION,
            Self::SessionNotes => DEFAULT_MAX_BYTES_NOTES,
            Self::Homebrew => DEFAULT_MAX_BYTES_HOMEBREW,
        }
    }

    /// Get the default semantic ratio for this content type.
    pub fn default_semantic_ratio(&self) -> f32 {
        match self {
            Self::Rules => DEFAULT_SEMANTIC_RATIO_RULES,
            Self::Fiction => DEFAULT_SEMANTIC_RATIO_FICTION,
            Self::SessionNotes => DEFAULT_SEMANTIC_RATIO_NOTES,
            Self::Homebrew => DEFAULT_SEMANTIC_RATIO_RULES, // Similar to rules
        }
    }

    /// Get the Liquid template for rendering documents from this index.
    pub fn default_template(&self) -> &'static str {
        match self {
            Self::Rules => RULES_TEMPLATE,
            Self::Fiction => FICTION_TEMPLATE,
            Self::SessionNotes => CHUNK_TEMPLATE,
            Self::Homebrew => CHUNK_TEMPLATE,
        }
    }

    /// Get all known TTRPG indexes.
    pub fn all() -> &'static [TtrpgIndex] {
        &[
            Self::Rules,
            Self::Fiction,
            Self::SessionNotes,
            Self::Homebrew,
        ]
    }
}

impl std::fmt::Display for TtrpgIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.index_uid())
    }
}

// ============================================================================
// RAG Configuration
// ============================================================================

/// Settings for configuring RAG behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagSettings {
    /// Default semantic ratio for hybrid search
    #[serde(default = "default_semantic_ratio")]
    pub semantic_ratio: f32,
    /// Maximum documents to retrieve per query
    #[serde(default = "default_document_limit")]
    pub document_limit: usize,
    /// Maximum bytes per document
    #[serde(default = "default_max_bytes")]
    pub max_bytes: usize,
}

fn default_semantic_ratio() -> f32 {
    DEFAULT_SEMANTIC_RATIO_RULES
}

fn default_document_limit() -> usize {
    DEFAULT_DOCUMENT_LIMIT
}

fn default_max_bytes() -> usize {
    DEFAULT_MAX_BYTES_RULES
}

impl Default for RagSettings {
    fn default() -> Self {
        Self {
            semantic_ratio: DEFAULT_SEMANTIC_RATIO_RULES,
            document_limit: DEFAULT_DOCUMENT_LIMIT,
            max_bytes: DEFAULT_MAX_BYTES_RULES,
        }
    }
}

/// TTRPG-focused RAG configuration wrapping `ChatConfig`.
///
/// Provides sensible defaults for tabletop RPG content retrieval while
/// allowing full customization.
#[derive(Debug, Clone)]
pub struct RagConfig {
    inner: ChatConfig,
}

impl RagConfig {
    /// Create a new RagConfig from an existing ChatConfig.
    pub fn from_inner(config: ChatConfig) -> Self {
        Self { inner: config }
    }

    /// Get a reference to the inner ChatConfig.
    pub fn inner(&self) -> &ChatConfig {
        &self.inner
    }

    /// Consume self and return the inner ChatConfig.
    pub fn into_inner(self) -> ChatConfig {
        self.inner
    }

    /// Get the configured LLM source.
    pub fn source(&self) -> &ChatSource {
        &self.inner.source
    }

    /// Get the configured model.
    pub fn model(&self) -> &str {
        &self.inner.model
    }

    /// Check if an index is configured.
    pub fn has_index(&self, index: &TtrpgIndex) -> bool {
        self.inner.index_configs.contains_key(index.index_uid())
    }

    /// Get index configuration for a specific TTRPG index.
    pub fn get_index_config(&self, index: &TtrpgIndex) -> Option<&ChatIndexConfig> {
        self.inner.index_configs.get(index.index_uid())
    }
}

impl From<RagConfig> for ChatConfig {
    fn from(config: RagConfig) -> Self {
        config.inner
    }
}

impl AsRef<ChatConfig> for RagConfig {
    fn as_ref(&self) -> &ChatConfig {
        &self.inner
    }
}

// ============================================================================
// Configuration Builder
// ============================================================================

/// Build a TTRPG-optimized ChatConfig from an LLM provider configuration.
pub fn build_ttrpg_chat_config(provider: LlmProvider, api_key: &str, model: &str) -> ChatConfig {
    let (source, base_url, deployment_id, api_version) = match &provider {
        LlmProvider::Anthropic => (ChatSource::Anthropic, None, None, None),
        LlmProvider::OpenAi => (ChatSource::OpenAi, None, None, None),
        LlmProvider::Mistral => (ChatSource::Mistral, None, None, None),
        LlmProvider::VLlm { base_url } => {
            (ChatSource::VLlm, Some(base_url.clone()), None, None)
        }
        LlmProvider::Ollama { base_url } => {
            (ChatSource::VLlm, Some(base_url.clone()), None, None)
        }
        LlmProvider::Azure {
            base_url,
            deployment_id,
            api_version,
        } => (
            ChatSource::AzureOpenAi,
            Some(base_url.clone()),
            Some(deployment_id.clone()),
            Some(api_version.clone()),
        ),
    };

    let prompts = ChatPrompts {
        system: Some(TTRPG_SYSTEM_PROMPT.to_string()),
        search_description: None,
        search_q_param: None,
        search_filter_param: None,
        search_index_uid_param: None,
    };

    ChatConfig {
        source,
        api_key: if provider.is_local() && api_key.is_empty() {
            "placeholder".to_string()
        } else {
            api_key.to_string()
        },
        base_url,
        model: model.to_string(),
        org_id: None,
        project_id: None,
        api_version,
        deployment_id,
        prompts,
        index_configs: create_default_index_configs(),
    }
}

/// Create default index configurations for all TTRPG indexes.
pub fn create_default_index_configs() -> HashMap<String, ChatIndexConfig> {
    TtrpgIndex::all()
        .iter()
        .map(|index| (index.index_uid().to_string(), create_index_config(*index)))
        .collect()
}

/// Create a ChatIndexConfig for a specific TTRPG index with appropriate defaults.
pub fn create_index_config(index: TtrpgIndex) -> ChatIndexConfig {
    ChatIndexConfig {
        description: index.description().to_string(),
        template: Some(index.default_template().to_string()),
        max_bytes: Some(index.default_max_bytes()),
        search_params: Some(ChatSearchParams {
            limit: Some(DEFAULT_DOCUMENT_LIMIT),
            sort: None,
            matching_strategy: Some("last".to_string()),
            semantic_ratio: Some(index.default_semantic_ratio()),
            embedder: Some("default".to_string()),
        }),
    }
}

/// Create a ChatIndexConfig with custom parameters.
pub fn create_custom_index_config(
    description: impl Into<String>,
    template: Option<String>,
    max_bytes: usize,
    semantic_ratio: f32,
    limit: usize,
) -> ChatIndexConfig {
    ChatIndexConfig {
        description: description.into(),
        template,
        max_bytes: Some(max_bytes),
        search_params: Some(ChatSearchParams {
            limit: Some(limit),
            sort: None,
            matching_strategy: Some("last".to_string()),
            semantic_ratio: Some(semantic_ratio),
            embedder: Some("default".to_string()),
        }),
    }
}

// ============================================================================
// Builder Pattern (Alternative API)
// ============================================================================

/// Builder for constructing TTRPG RAG configurations.
///
/// # Example
///
/// ```rust,ignore
/// let config = RagConfigBuilder::new()
///     .with_anthropic("sk-ant-...", "claude-3-5-sonnet-20241022")
///     .with_default_index_configs()
///     .build()?;
/// ```
#[derive(Debug, Default)]
pub struct RagConfigBuilder {
    source: Option<ChatSource>,
    api_key: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
    org_id: Option<String>,
    project_id: Option<String>,
    api_version: Option<String>,
    deployment_id: Option<String>,
    system_prompt: Option<String>,
    search_description: Option<String>,
    index_configs: HashMap<String, ChatIndexConfig>,
}

/// Error type for RAG configuration building.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RagConfigError {
    /// No LLM provider configured.
    MissingProvider,
    /// No API key provided (required for non-Ollama providers).
    MissingApiKey,
    /// No model specified.
    MissingModel,
    /// Invalid configuration.
    InvalidConfig(String),
}

impl std::fmt::Display for RagConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingProvider => write!(f, "No LLM provider configured"),
            Self::MissingApiKey => write!(f, "API key required for this provider"),
            Self::MissingModel => write!(f, "Model name required"),
            Self::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
        }
    }
}

impl std::error::Error for RagConfigError {}

impl RagConfigBuilder {
    /// Create a new builder with no configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure Anthropic Claude as the LLM provider.
    pub fn with_anthropic(mut self, api_key: impl Into<String>, model: impl Into<String>) -> Self {
        self.source = Some(ChatSource::Anthropic);
        self.api_key = Some(api_key.into());
        self.model = Some(model.into());
        self
    }

    /// Configure OpenAI as the LLM provider.
    pub fn with_openai(mut self, api_key: impl Into<String>, model: impl Into<String>) -> Self {
        self.source = Some(ChatSource::OpenAi);
        self.api_key = Some(api_key.into());
        self.model = Some(model.into());
        self
    }

    /// Configure OpenAI with organization ID.
    pub fn with_openai_org(
        mut self,
        api_key: impl Into<String>,
        model: impl Into<String>,
        org_id: impl Into<String>,
    ) -> Self {
        self.source = Some(ChatSource::OpenAi);
        self.api_key = Some(api_key.into());
        self.model = Some(model.into());
        self.org_id = Some(org_id.into());
        self
    }

    /// Configure Ollama (local LLM) as the provider.
    pub fn with_ollama(mut self, base_url: impl Into<String>, model: impl Into<String>) -> Self {
        self.source = Some(ChatSource::VLlm);
        self.base_url = Some(base_url.into());
        self.model = Some(model.into());
        self.api_key = Some("ollama".to_string());
        self
    }

    /// Configure Mistral as the LLM provider.
    pub fn with_mistral(mut self, api_key: impl Into<String>, model: impl Into<String>) -> Self {
        self.source = Some(ChatSource::Mistral);
        self.api_key = Some(api_key.into());
        self.model = Some(model.into());
        self
    }

    /// Configure Azure OpenAI as the LLM provider.
    pub fn with_azure_openai(
        mut self,
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        deployment_id: impl Into<String>,
        api_version: impl Into<String>,
    ) -> Self {
        self.source = Some(ChatSource::AzureOpenAi);
        self.api_key = Some(api_key.into());
        self.base_url = Some(base_url.into());
        self.deployment_id = Some(deployment_id.into());
        self.api_version = Some(api_version.into());
        self.model = Some("azure".to_string());
        self
    }

    /// Configure a vLLM-compatible endpoint.
    pub fn with_vllm(mut self, base_url: impl Into<String>, model: impl Into<String>) -> Self {
        self.source = Some(ChatSource::VLlm);
        self.base_url = Some(base_url.into());
        self.model = Some(model.into());
        self.api_key = Some("vllm".to_string());
        self
    }

    /// Override the default system prompt.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set a custom search function description for the LLM.
    pub fn with_search_description(mut self, description: impl Into<String>) -> Self {
        self.search_description = Some(description.into());
        self
    }

    /// Add default TTRPG index configurations for all known indexes.
    pub fn with_default_index_configs(mut self) -> Self {
        for index in TtrpgIndex::all() {
            self.index_configs.insert(
                index.index_uid().to_string(),
                create_index_config(*index),
            );
        }
        self
    }

    /// Add a specific TTRPG index with default configuration.
    pub fn with_ttrpg_index(mut self, index: TtrpgIndex) -> Self {
        self.index_configs.insert(
            index.index_uid().to_string(),
            create_index_config(index),
        );
        self
    }

    /// Add a custom index configuration.
    pub fn with_index_config(
        mut self,
        index_uid: impl Into<String>,
        config: ChatIndexConfig,
    ) -> Self {
        self.index_configs.insert(index_uid.into(), config);
        self
    }

    /// Build the RAG configuration.
    pub fn build(self) -> Result<RagConfig, RagConfigError> {
        let source = self.source.ok_or(RagConfigError::MissingProvider)?;
        let api_key = self.api_key.ok_or(RagConfigError::MissingApiKey)?;
        let model = self.model.ok_or(RagConfigError::MissingModel)?;

        let prompts = ChatPrompts {
            system: Some(
                self.system_prompt
                    .unwrap_or_else(|| TTRPG_SYSTEM_PROMPT.to_string()),
            ),
            search_description: self.search_description,
            search_q_param: None,
            search_filter_param: None,
            search_index_uid_param: None,
        };

        let config = ChatConfig {
            source,
            api_key,
            base_url: self.base_url,
            model,
            org_id: self.org_id,
            project_id: self.project_id,
            api_version: self.api_version,
            deployment_id: self.deployment_id,
            prompts,
            index_configs: self.index_configs,
        };

        Ok(RagConfig::from_inner(config))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ttrpg_index_uids() {
        assert_eq!(TtrpgIndex::Rules.index_uid(), "ttrpg_rules");
        assert_eq!(TtrpgIndex::Fiction.index_uid(), "ttrpg_fiction");
        assert_eq!(TtrpgIndex::SessionNotes.index_uid(), "session_notes");
        assert_eq!(TtrpgIndex::Homebrew.index_uid(), "homebrew");
    }

    #[test]
    fn test_ttrpg_index_descriptions() {
        for index in TtrpgIndex::all() {
            assert!(!index.description().is_empty());
            assert!(index.description().len() > 20);
        }
    }

    #[test]
    fn test_build_ttrpg_chat_config_anthropic() {
        let config = build_ttrpg_chat_config(
            LlmProvider::Anthropic,
            "test-key",
            "claude-3-5-sonnet-20241022",
        );

        assert_eq!(config.source, ChatSource::Anthropic);
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.model, "claude-3-5-sonnet-20241022");
        assert!(config.prompts.system.is_some());
        assert!(!config.index_configs.is_empty());
    }

    #[test]
    fn test_build_ttrpg_chat_config_ollama() {
        let config = build_ttrpg_chat_config(
            LlmProvider::Ollama {
                base_url: "http://localhost:11434".to_string(),
            },
            "",
            "llama3.2",
        );

        assert_eq!(config.source, ChatSource::VLlm);
        assert_eq!(config.api_key, "placeholder");
        assert!(config.base_url.is_some());
    }

    #[test]
    fn test_builder_anthropic() {
        let config = RagConfigBuilder::new()
            .with_anthropic("sk-ant-test", "claude-3-5-sonnet-20241022")
            .with_default_index_configs()
            .build()
            .expect("Should build successfully");

        assert_eq!(*config.source(), ChatSource::Anthropic);
        assert_eq!(config.model(), "claude-3-5-sonnet-20241022");
        assert!(config.has_index(&TtrpgIndex::Rules));
        assert!(config.has_index(&TtrpgIndex::Fiction));
    }

    #[test]
    fn test_builder_openai() {
        let config = RagConfigBuilder::new()
            .with_openai("sk-test", "gpt-4o")
            .with_ttrpg_index(TtrpgIndex::Rules)
            .build()
            .expect("Should build successfully");

        assert_eq!(*config.source(), ChatSource::OpenAi);
        assert!(config.has_index(&TtrpgIndex::Rules));
        assert!(!config.has_index(&TtrpgIndex::Fiction));
    }

    #[test]
    fn test_builder_ollama() {
        let config = RagConfigBuilder::new()
            .with_ollama("http://localhost:11434", "llama3.2")
            .build()
            .expect("Should build successfully");

        assert_eq!(*config.source(), ChatSource::VLlm);
        assert_eq!(config.model(), "llama3.2");
        assert!(config.inner().base_url.is_some());
    }

    #[test]
    fn test_builder_custom_system_prompt() {
        let custom_prompt = "You are a D&D 5e expert.";
        let config = RagConfigBuilder::new()
            .with_anthropic("key", "model")
            .with_system_prompt(custom_prompt)
            .build()
            .expect("Should build successfully");

        assert_eq!(config.inner().prompts.system.as_deref(), Some(custom_prompt));
    }

    #[test]
    fn test_builder_missing_provider() {
        let result = RagConfigBuilder::new().build();
        assert!(matches!(result, Err(RagConfigError::MissingProvider)));
    }

    #[test]
    fn test_create_default_index_configs() {
        let configs = create_default_index_configs();
        assert_eq!(configs.len(), TtrpgIndex::all().len());

        for index in TtrpgIndex::all() {
            let config = configs.get(index.index_uid()).expect("Index should exist");
            assert!(!config.description.is_empty());
            assert!(config.max_bytes.is_some());
            assert!(config.search_params.is_some());
        }
    }

    #[test]
    fn test_create_index_config() {
        let config = create_index_config(TtrpgIndex::Rules);

        assert!(config.description.contains("rules"));
        assert_eq!(config.max_bytes, Some(DEFAULT_MAX_BYTES_RULES));

        let params = config.search_params.expect("Should have search params");
        assert_eq!(params.semantic_ratio, Some(DEFAULT_SEMANTIC_RATIO_RULES));
        assert_eq!(params.limit, Some(DEFAULT_DOCUMENT_LIMIT));
    }

    #[test]
    fn test_create_custom_index_config() {
        let config = create_custom_index_config(
            "Custom campaign index",
            Some("{{ title }}: {{ content }}".to_string()),
            500,
            0.8,
            5,
        );

        assert_eq!(config.description, "Custom campaign index");
        assert_eq!(config.max_bytes, Some(500));

        let params = config.search_params.expect("Should have search params");
        assert_eq!(params.semantic_ratio, Some(0.8));
        assert_eq!(params.limit, Some(5));
    }

    #[test]
    fn test_rag_config_into_chat_config() {
        let rag_config = RagConfigBuilder::new()
            .with_anthropic("key", "model")
            .build()
            .expect("Should build");

        let chat_config: ChatConfig = rag_config.into();
        assert_eq!(chat_config.source, ChatSource::Anthropic);
    }

    #[test]
    fn test_ttrpg_index_display() {
        assert_eq!(format!("{}", TtrpgIndex::Rules), "ttrpg_rules");
        assert_eq!(format!("{}", TtrpgIndex::Fiction), "ttrpg_fiction");
    }

    #[test]
    fn test_semantic_ratio_ranges() {
        for index in TtrpgIndex::all() {
            let ratio = index.default_semantic_ratio();
            assert!(ratio >= 0.0 && ratio <= 1.0, "Ratio should be 0.0-1.0");
        }
    }

    #[test]
    fn test_max_bytes_reasonable() {
        for index in TtrpgIndex::all() {
            let max_bytes = index.default_max_bytes();
            assert!(
                max_bytes >= 400 && max_bytes <= 2000,
                "Max bytes should be reasonable for LLM context"
            );
        }
    }

    #[test]
    fn test_builder_azure_openai() {
        let config = RagConfigBuilder::new()
            .with_azure_openai(
                "azure-key",
                "https://my-resource.openai.azure.com",
                "gpt-4-deployment",
                "2024-02-15-preview",
            )
            .build()
            .expect("Should build successfully");

        assert_eq!(*config.source(), ChatSource::AzureOpenAi);
        assert!(config.inner().base_url.is_some());
        assert!(config.inner().deployment_id.is_some());
        assert!(config.inner().api_version.is_some());
    }

    #[test]
    fn test_builder_mistral() {
        let config = RagConfigBuilder::new()
            .with_mistral("mistral-key", "mistral-large-latest")
            .build()
            .expect("Should build successfully");

        assert_eq!(*config.source(), ChatSource::Mistral);
        assert_eq!(config.model(), "mistral-large-latest");
    }

    #[test]
    fn test_rag_config_error_display() {
        assert_eq!(
            RagConfigError::MissingProvider.to_string(),
            "No LLM provider configured"
        );
        assert_eq!(
            RagConfigError::MissingApiKey.to_string(),
            "API key required for this provider"
        );
        assert_eq!(
            RagConfigError::InvalidConfig("test".to_string()).to_string(),
            "Invalid configuration: test"
        );
    }

    #[test]
    fn test_rag_settings_default() {
        let settings = RagSettings::default();
        assert_eq!(settings.semantic_ratio, DEFAULT_SEMANTIC_RATIO_RULES);
        assert_eq!(settings.document_limit, DEFAULT_DOCUMENT_LIMIT);
        assert_eq!(settings.max_bytes, DEFAULT_MAX_BYTES_RULES);
    }
}
