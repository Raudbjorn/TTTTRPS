//! RAG Type Definitions
//!
//! Frontend-facing types for RAG configuration and query payloads.
//! These types provide a stable API for the frontend while internally
//! converting to/from meilisearch_lib types.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ============================================================================
// LLM Provider Configuration
// ============================================================================

/// LLM provider source.
///
/// Matches meilisearch_lib::ChatSource but uses snake_case for frontend compatibility.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RagProviderSource {
    /// OpenAI API (GPT-4, GPT-3.5, etc.)
    OpenAi,
    /// Anthropic Claude API
    Anthropic,
    /// Azure OpenAI Service
    AzureOpenAi,
    /// Mistral AI API
    Mistral,
    /// vLLM server (self-hosted)
    VLlm,
}

impl From<RagProviderSource> for meilisearch_lib::ChatSource {
    fn from(source: RagProviderSource) -> Self {
        match source {
            RagProviderSource::OpenAi => meilisearch_lib::ChatSource::OpenAi,
            RagProviderSource::Anthropic => meilisearch_lib::ChatSource::Anthropic,
            RagProviderSource::AzureOpenAi => meilisearch_lib::ChatSource::AzureOpenAi,
            RagProviderSource::Mistral => meilisearch_lib::ChatSource::Mistral,
            RagProviderSource::VLlm => meilisearch_lib::ChatSource::VLlm,
        }
    }
}

impl From<meilisearch_lib::ChatSource> for RagProviderSource {
    fn from(source: meilisearch_lib::ChatSource) -> Self {
        match source {
            meilisearch_lib::ChatSource::OpenAi => RagProviderSource::OpenAi,
            meilisearch_lib::ChatSource::Anthropic => RagProviderSource::Anthropic,
            meilisearch_lib::ChatSource::AzureOpenAi => RagProviderSource::AzureOpenAi,
            meilisearch_lib::ChatSource::Mistral => RagProviderSource::Mistral,
            meilisearch_lib::ChatSource::VLlm => RagProviderSource::VLlm,
        }
    }
}

/// RAG prompt configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RagPromptsPayload {
    /// System prompt for the LLM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Description of search function capabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_description: Option<String>,
    /// Instructions for query parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_q_param: Option<String>,
    /// Instructions for filter parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_filter_param: Option<String>,
    /// Instructions for index selection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_index_uid_param: Option<String>,
}

impl From<RagPromptsPayload> for meilisearch_lib::ChatPrompts {
    fn from(prompts: RagPromptsPayload) -> Self {
        meilisearch_lib::ChatPrompts {
            system: prompts.system,
            search_description: prompts.search_description,
            search_q_param: prompts.search_q_param,
            search_filter_param: prompts.search_filter_param,
            search_index_uid_param: prompts.search_index_uid_param,
        }
    }
}

impl From<meilisearch_lib::ChatPrompts> for RagPromptsPayload {
    fn from(prompts: meilisearch_lib::ChatPrompts) -> Self {
        RagPromptsPayload {
            system: prompts.system,
            search_description: prompts.search_description,
            search_q_param: prompts.search_q_param,
            search_filter_param: prompts.search_filter_param,
            search_index_uid_param: prompts.search_index_uid_param,
        }
    }
}

/// Search parameters for context retrieval.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagSearchParamsPayload {
    /// Maximum number of documents to retrieve.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    /// Sort criteria.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<Vec<String>>,
    /// Matching strategy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matching_strategy: Option<String>,
    /// Semantic ratio for hybrid search (0.0 = keyword only, 1.0 = semantic only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_ratio: Option<f32>,
    /// Which embedder to use for semantic search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedder: Option<String>,
}

impl From<RagSearchParamsPayload> for meilisearch_lib::ChatSearchParams {
    fn from(params: RagSearchParamsPayload) -> Self {
        meilisearch_lib::ChatSearchParams {
            limit: params.limit,
            sort: params.sort,
            matching_strategy: params.matching_strategy,
            semantic_ratio: params.semantic_ratio,
            embedder: params.embedder,
        }
    }
}

impl From<meilisearch_lib::ChatSearchParams> for RagSearchParamsPayload {
    fn from(params: meilisearch_lib::ChatSearchParams) -> Self {
        RagSearchParamsPayload {
            limit: params.limit,
            sort: params.sort,
            matching_strategy: params.matching_strategy,
            semantic_ratio: params.semantic_ratio,
            embedder: params.embedder,
        }
    }
}

/// Per-index RAG configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagIndexConfigPayload {
    /// Human-readable description of index contents.
    pub description: String,
    /// Liquid template for rendering documents in context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    /// Maximum bytes per document in context (default: 400).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_bytes: Option<usize>,
    /// Search parameters for this index.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_params: Option<RagSearchParamsPayload>,
}

impl From<RagIndexConfigPayload> for meilisearch_lib::ChatIndexConfig {
    fn from(config: RagIndexConfigPayload) -> Self {
        meilisearch_lib::ChatIndexConfig {
            description: config.description,
            template: config.template,
            max_bytes: config.max_bytes,
            search_params: config.search_params.map(Into::into),
        }
    }
}

impl From<meilisearch_lib::ChatIndexConfig> for RagIndexConfigPayload {
    fn from(config: meilisearch_lib::ChatIndexConfig) -> Self {
        RagIndexConfigPayload {
            description: config.description,
            template: config.template,
            max_bytes: config.max_bytes,
            search_params: config.search_params.map(Into::into),
        }
    }
}

/// Complete RAG configuration payload for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagConfigPayload {
    /// LLM provider source.
    pub source: RagProviderSource,
    /// API key for the provider.
    /// When returned from `get_rag_config`, this is masked.
    pub api_key: String,
    /// Custom base URL (required for Azure, vLLM; optional for others).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// Model identifier (e.g., "gpt-4", "claude-3-sonnet-20240229").
    pub model: String,
    /// Organization ID (OpenAI only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_id: Option<String>,
    /// Project ID (OpenAI only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    /// API version (Azure only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
    /// Deployment ID (Azure only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_id: Option<String>,
    /// Prompt configuration.
    #[serde(default)]
    pub prompts: RagPromptsPayload,
    /// Per-index chat configuration.
    #[serde(default)]
    pub index_configs: HashMap<String, RagIndexConfigPayload>,
}

impl RagConfigPayload {
    /// Mask the API key for safe return to frontend.
    ///
    /// Shows first 4 and last 4 characters with asterisks in between.
    pub fn with_masked_api_key(mut self) -> Self {
        self.api_key = mask_api_key(&self.api_key);
        self
    }
}

impl From<RagConfigPayload> for meilisearch_lib::ChatConfig {
    fn from(config: RagConfigPayload) -> Self {
        meilisearch_lib::ChatConfig {
            source: config.source.into(),
            api_key: config.api_key,
            base_url: config.base_url,
            model: config.model,
            org_id: config.org_id,
            project_id: config.project_id,
            api_version: config.api_version,
            deployment_id: config.deployment_id,
            prompts: config.prompts.into(),
            index_configs: config.index_configs.into_iter().map(|(k, v)| (k, v.into())).collect(),
        }
    }
}

impl From<meilisearch_lib::ChatConfig> for RagConfigPayload {
    fn from(config: meilisearch_lib::ChatConfig) -> Self {
        RagConfigPayload {
            source: config.source.into(),
            api_key: config.api_key,
            base_url: config.base_url,
            model: config.model,
            org_id: config.org_id,
            project_id: config.project_id,
            api_version: config.api_version,
            deployment_id: config.deployment_id,
            prompts: config.prompts.into(),
            index_configs: config.index_configs.into_iter().map(|(k, v)| (k, v.into())).collect(),
        }
    }
}

/// Mask an API key for display.
fn mask_api_key(key: &str) -> String {
    if key.len() <= 8 {
        "*".repeat(key.len())
    } else {
        let prefix = &key[..4];
        let suffix = &key[key.len() - 4..];
        format!("{}{}{}",prefix, "*".repeat(key.len() - 8), suffix)
    }
}

// ============================================================================
// Message Types
// ============================================================================

/// Message role in conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RagMessageRole {
    /// System message (instructions).
    System,
    /// User message.
    User,
    /// Assistant response.
    Assistant,
    /// Tool result.
    Tool,
}

impl From<RagMessageRole> for meilisearch_lib::Role {
    fn from(role: RagMessageRole) -> Self {
        match role {
            RagMessageRole::System => meilisearch_lib::Role::System,
            RagMessageRole::User => meilisearch_lib::Role::User,
            RagMessageRole::Assistant => meilisearch_lib::Role::Assistant,
            RagMessageRole::Tool => meilisearch_lib::Role::Tool,
        }
    }
}

impl From<meilisearch_lib::Role> for RagMessageRole {
    fn from(role: meilisearch_lib::Role) -> Self {
        match role {
            meilisearch_lib::Role::System => RagMessageRole::System,
            meilisearch_lib::Role::User => RagMessageRole::User,
            meilisearch_lib::Role::Assistant => RagMessageRole::Assistant,
            meilisearch_lib::Role::Tool => RagMessageRole::Tool,
        }
    }
}

/// Chat message from frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagMessagePayload {
    /// Message role.
    pub role: RagMessageRole,
    /// Message content.
    pub content: String,
    /// Tool call ID (for tool messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl From<RagMessagePayload> for meilisearch_lib::Message {
    fn from(msg: RagMessagePayload) -> Self {
        meilisearch_lib::Message {
            role: msg.role.into(),
            content: msg.content,
            tool_call_id: msg.tool_call_id,
        }
    }
}

// ============================================================================
// Response Types
// ============================================================================

/// Token usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagUsagePayload {
    /// Tokens in the prompt.
    pub prompt_tokens: u32,
    /// Tokens in the completion.
    pub completion_tokens: u32,
    /// Total tokens used.
    pub total_tokens: u32,
}

impl From<meilisearch_lib::Usage> for RagUsagePayload {
    fn from(usage: meilisearch_lib::Usage) -> Self {
        RagUsagePayload {
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
        }
    }
}

/// Source citation from RAG response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagSourcePayload {
    /// Document ID.
    pub id: String,
}

/// Non-streaming RAG response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagResponsePayload {
    /// Generated response content.
    pub content: String,
    /// Source document citations.
    pub sources: Vec<RagSourcePayload>,
    /// Token usage statistics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<RagUsagePayload>,
}

impl From<meilisearch_lib::ChatResponse> for RagResponsePayload {
    fn from(response: meilisearch_lib::ChatResponse) -> Self {
        RagResponsePayload {
            content: response.content,
            sources: response.sources.into_iter().map(|id| RagSourcePayload { id }).collect(),
            usage: response.usage.map(Into::into),
        }
    }
}

/// Streaming RAG chunk event.
///
/// Emitted as 'rag-chunk' Tauri events during streaming queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagChunkPayload {
    /// Stream identifier for matching chunks to requests.
    pub stream_id: String,
    /// Incremental content delta.
    pub delta: String,
    /// Whether this is the final chunk.
    pub done: bool,
    /// Source citations (present in first chunk only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sources: Option<Vec<RagSourcePayload>>,
    /// Chunk index for ordering.
    pub index: u32,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_api_key_short() {
        assert_eq!(mask_api_key("abc"), "***");
        assert_eq!(mask_api_key("12345678"), "********");
    }

    #[test]
    fn test_mask_api_key_long() {
        // "sk-12345678901234567890" is 23 chars: prefix 4, asterisks 15, suffix 4
        assert_eq!(mask_api_key("sk-12345678901234567890"), "sk-1***************7890");
        // "123456789" is 9 chars: prefix 4, asterisks 1, suffix 4
        assert_eq!(mask_api_key("123456789"), "1234*6789");
    }

    #[test]
    fn test_rag_provider_source_serialization() {
        let source = RagProviderSource::OpenAi;
        let json = serde_json::to_string(&source).unwrap();
        assert_eq!(json, "\"open_ai\"");

        let anthropic = RagProviderSource::Anthropic;
        let json = serde_json::to_string(&anthropic).unwrap();
        assert_eq!(json, "\"anthropic\"");
    }

    #[test]
    fn test_rag_provider_source_conversion() {
        let rag_source = RagProviderSource::Anthropic;
        let meili_source: meilisearch_lib::ChatSource = rag_source.into();
        assert_eq!(meili_source, meilisearch_lib::ChatSource::Anthropic);

        let back: RagProviderSource = meili_source.into();
        assert_eq!(back, RagProviderSource::Anthropic);
    }

    #[test]
    fn test_rag_message_role_serialization() {
        let role = RagMessageRole::User;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"user\"");
    }

    #[test]
    fn test_rag_config_payload_with_masked_key() {
        let config = RagConfigPayload {
            source: RagProviderSource::OpenAi,
            api_key: "sk-12345678901234567890".to_string(),
            base_url: None,
            model: "gpt-4".to_string(),
            org_id: None,
            project_id: None,
            api_version: None,
            deployment_id: None,
            prompts: RagPromptsPayload::default(),
            index_configs: HashMap::new(),
        };

        let masked = config.with_masked_api_key();
        assert!(masked.api_key.starts_with("sk-1"));
        assert!(masked.api_key.ends_with("7890"));
        assert!(masked.api_key.contains("*"));
    }

    #[test]
    fn test_rag_response_payload_serialization() {
        let response = RagResponsePayload {
            content: "Hello!".to_string(),
            sources: vec![
                RagSourcePayload { id: "doc1".to_string() },
                RagSourcePayload { id: "doc2".to_string() },
            ],
            usage: Some(RagUsagePayload {
                prompt_tokens: 100,
                completion_tokens: 50,
                total_tokens: 150,
            }),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"content\":\"Hello!\""));
        assert!(json.contains("\"promptTokens\":100"));
    }

    #[test]
    fn test_rag_chunk_payload_serialization() {
        let chunk = RagChunkPayload {
            stream_id: "stream-123".to_string(),
            delta: "Hello".to_string(),
            done: false,
            sources: Some(vec![RagSourcePayload { id: "doc1".to_string() }]),
            index: 1,
        };

        let json = serde_json::to_string(&chunk).unwrap();
        assert!(json.contains("\"streamId\":\"stream-123\""));
        assert!(json.contains("\"delta\":\"Hello\""));
        assert!(json.contains("\"done\":false"));
    }

    #[test]
    fn test_rag_message_payload_to_meili() {
        let payload = RagMessagePayload {
            role: RagMessageRole::User,
            content: "Hello".to_string(),
            tool_call_id: None,
        };

        let meili_msg: meilisearch_lib::Message = payload.into();
        assert_eq!(meili_msg.role, meilisearch_lib::Role::User);
        assert_eq!(meili_msg.content, "Hello");
    }
}
