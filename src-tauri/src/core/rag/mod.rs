//! RAG (Retrieval-Augmented Generation) Configuration Module
//!
//! This module provides TTRPG-specific RAG configuration for the embedded
//! Meilisearch search engine. It defines configurations for LLM providers,
//! index-specific settings, and Liquid templates for document formatting.
//!
//! # Components
//!
//! - `RagConfig`: Main configuration wrapper for `meilisearch_lib::ChatConfig`
//! - `RagConfigBuilder`: Builder pattern for flexible configuration
//! - `TtrpgIndex`: Enum of known TTRPG content indexes
//! - `LlmProvider`: Enum of supported LLM providers (Anthropic, OpenAI, Ollama, etc.)
//! - TTRPG-specific templates for rules, fiction, and chunk formatting
//!
//! # Usage
//!
//! ## Quick Configuration
//!
//! ```rust,ignore
//! use crate::core::rag::{build_ttrpg_chat_config, LlmProvider};
//!
//! let config = build_ttrpg_chat_config(
//!     LlmProvider::Anthropic,
//!     "api-key",
//!     "claude-sonnet-4-20250514"
//! );
//!
//! // Use with EmbeddedSearch
//! search.inner().set_chat_config(Some(config));
//! ```
//!
//! ## Builder Pattern
//!
//! ```rust,ignore
//! use crate::core::rag::{RagConfigBuilder, TtrpgIndex};
//!
//! let config = RagConfigBuilder::new()
//!     .with_anthropic("sk-ant-...", "claude-3-5-sonnet-20241022")
//!     .with_system_prompt("Custom GM assistant prompt")
//!     .with_default_index_configs()
//!     .build()?;
//!
//! search.inner().set_chat_config(Some(config.into_inner()));
//! ```

mod config;
mod provider;
mod templates;

// Core configuration types
pub use config::{
    build_ttrpg_chat_config, create_custom_index_config, create_default_index_configs,
    create_index_config, RagConfig, RagConfigBuilder, RagConfigError, RagSettings, TtrpgIndex,
    DEFAULT_DOCUMENT_LIMIT, DEFAULT_MAX_BYTES_FICTION, DEFAULT_MAX_BYTES_HOMEBREW,
    DEFAULT_MAX_BYTES_NOTES, DEFAULT_MAX_BYTES_RULES, DEFAULT_SEMANTIC_RATIO_FICTION,
    DEFAULT_SEMANTIC_RATIO_NOTES, DEFAULT_SEMANTIC_RATIO_RULES,
};

// Provider types
pub use provider::{parse_provider, LlmProvider};

// Template constants
pub use templates::{
    CAMPAIGN_CONTEXT_TEMPLATE, CHUNK_TEMPLATE, FICTION_TEMPLATE, NPC_TEMPLATE, RULES_TEMPLATE,
    TTRPG_SYSTEM_PROMPT,
};
