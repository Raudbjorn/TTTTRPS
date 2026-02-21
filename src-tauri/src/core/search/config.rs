//! Search Configuration
//!
//! Configuration types for search indexes and embedders.

use serde::{Deserialize, Serialize};

// ============================================================================
// Index Constants
// ============================================================================

/// Index for game rules, mechanics, and technical content
pub const INDEX_RULES: &str = "rules";
/// Index for fiction, lore, and narrative content
pub const INDEX_FICTION: &str = "fiction";
/// Index for chat/conversation history
pub const INDEX_CHAT: &str = "chat";
/// Index for general documents (user uploads)
pub const INDEX_DOCUMENTS: &str = "documents";
/// Index for library document metadata (persistence)
pub const INDEX_LIBRARY_METADATA: &str = "library_metadata";

// ============================================================================
// Timeout Constants
// ============================================================================

/// Default timeout for short operations (metadata updates, single deletes)
pub const TASK_TIMEOUT_SHORT_SECS: u64 = 30;
/// Default timeout for long operations (bulk ingestion, index creation)
pub const TASK_TIMEOUT_LONG_SECS: u64 = 600; // 10 minutes

// ============================================================================
// Embedder Configuration
// ============================================================================

/// Embedder provider type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "camelCase")]
pub enum EmbedderConfig {
    /// OpenAI embeddings
    #[serde(rename = "openAi")]
    OpenAI {
        #[serde(rename = "apiKey")]
        api_key: String,
        model: Option<String>,
        dimensions: Option<u32>,
    },
    /// Ollama embeddings (local) - uses Meilisearch's built-in ollama source
    #[serde(rename = "ollama")]
    Ollama { url: String, model: String },
    /// HuggingFace embeddings
    #[serde(rename = "huggingFace")]
    HuggingFace { model: String },
    /// REST-based Ollama embedder (for more control)
    #[serde(rename = "rest")]
    OllamaRest {
        /// Ollama host URL (e.g., "http://localhost:11434")
        host: String,
        /// Model name (e.g., "nomic-embed-text")
        model: String,
        /// Embedding dimensions for this model
        dimensions: u32,
    },
    /// GitHub Copilot embeddings
    #[serde(rename = "copilot")]
    Copilot {
        api_key: String,
        model: String,
        dimensions: u32,
    },
}

/// Get embedding dimensions for common Ollama models
pub fn ollama_embedding_dimensions(model: &str) -> u32 {
    match model {
        "nomic-embed-text" => 768,
        "mxbai-embed-large" => 1024,
        "all-minilm" | "all-minilm:l6-v2" => 384,
        "snowflake-arctic-embed" | "snowflake-arctic-embed:s" => 384,
        "snowflake-arctic-embed:m" => 768,
        "snowflake-arctic-embed:l" => 1024,
        "bge-m3" => 1024,
        "bge-large" => 1024,
        "bge-base" => 768,
        "bge-small" => 384,
        _ => 768, // Default fallback
    }
}

/// Get embedding dimensions for GitHub Copilot models
pub fn copilot_embedding_dimensions(model: &str) -> u32 {
    match model {
        "text-embedding-3-small" => 1536,
        "text-embedding-3-large" => 3072,
        _ => 1536,
    }
}

// ============================================================================
// Document Template for Embeddings
// ============================================================================

/// Rich document template for TTRPG content semantic embedding.
///
/// Uses Liquid templating with conditional fields for context-aware embedding.
/// Based on MDMAI patterns for optimal embedding quality.
///
/// Example output:
/// ```text
/// [STAT_BLOCK] [Delta Green] Agent's Handbook (rulebook, p.96) - Equipment > Weapons [combat] [cosmic horror]
///  Type: stat_block
///  Topics: firearms, damage, concealment
///  The standard handgun is a semi-automatic pistol...
/// ```
pub const TTRPG_DOCUMENT_TEMPLATE: &str = r#"{% if doc.chunk_type and doc.chunk_type != "text" and doc.chunk_type != "narrative" %}[{{ doc.chunk_type | upcase }}] {% endif %}{% if doc.game_system %}[{{ doc.game_system }}] {% endif %}{% if doc.book_title %}{{ doc.book_title }}{% else %}{{ doc.source }}{% endif %}{% if doc.content_category %} ({{ doc.content_category }}{% if doc.page_number %}, p.{{ doc.page_number }}{% endif %}){% elsif doc.page_number %} (p.{{ doc.page_number }}){% endif %}{% if doc.section_path %} - {{ doc.section_path }}{% elsif doc.chapter_title %} - {{ doc.chapter_title }}{% if doc.section_title %} > {{ doc.section_title }}{% endif %}{% if doc.subsection_title %} > {{ doc.subsection_title }}{% endif %}{% elsif doc.section_title %} - {{ doc.section_title }}{% endif %}{% if doc.mechanic_type %} [{{ doc.mechanic_type }}]{% endif %}{% if doc.genre %} [{{ doc.genre }}]{% endif %}
Type: {{ doc.chunk_type | default: "text" }}
{% if doc.semantic_keywords.size > 0 %}Topics: {{ doc.semantic_keywords | join: ", " }}
{% endif %}{{ doc.content | truncatewords: 300 }}"#;

/// Maximum bytes for document template output
pub const DOCUMENT_TEMPLATE_MAX_BYTES: u32 = 4000;

/// Build embedder settings JSON for a given configuration
pub fn build_embedder_json(config: &EmbedderConfig) -> serde_json::Value {
    match config {
        EmbedderConfig::OpenAI {
            api_key,
            model,
            dimensions,
        } => {
            serde_json::json!({
                "source": "openAi",
                "apiKey": api_key,
                "model": model.clone().unwrap_or_else(|| "text-embedding-3-small".to_string()),
                "dimensions": dimensions.unwrap_or(1536),
                "documentTemplate": TTRPG_DOCUMENT_TEMPLATE,
                "documentTemplateMaxBytes": DOCUMENT_TEMPLATE_MAX_BYTES
            })
        }
        EmbedderConfig::Ollama { url, model } => {
            serde_json::json!({
                "source": "ollama",
                "url": url,
                "model": model,
                "documentTemplate": TTRPG_DOCUMENT_TEMPLATE,
                "documentTemplateMaxBytes": DOCUMENT_TEMPLATE_MAX_BYTES
            })
        }
        EmbedderConfig::HuggingFace { model } => {
            serde_json::json!({
                "source": "huggingFace",
                "model": model,
                "documentTemplate": TTRPG_DOCUMENT_TEMPLATE,
                "documentTemplateMaxBytes": DOCUMENT_TEMPLATE_MAX_BYTES
            })
        }
        EmbedderConfig::OllamaRest {
            host,
            model,
            dimensions,
        } => {
            // REST-based Ollama embedder configuration
            // Ollama's embedding API: POST /api/embeddings with {"model": "...", "prompt": "..."}
            // Response: {"embedding": [...]}
            serde_json::json!({
                "source": "rest",
                "url": format!("{}/api/embeddings", host),
                "request": {
                    "model": model,
                    "prompt": "{{text}}"
                },
                "response": {
                    "embedding": "{{embedding}}"
                },
                "dimensions": dimensions,
                "documentTemplate": TTRPG_DOCUMENT_TEMPLATE,
                "documentTemplateMaxBytes": DOCUMENT_TEMPLATE_MAX_BYTES
            })
        }
        EmbedderConfig::Copilot {
            api_key,
            model,
            dimensions,
        } => {
            serde_json::json!({
                "source": "rest",
                "url": "https://api.githubcopilot.com/embeddings",
                "request": {
                    "model": model,
                    "input": "{{text}}"
                },
                "response": {
                    "embedding": "{{embedding}}"
                },
                "headers": {
                    "Authorization": format!("Bearer {}", api_key)
                },
                "dimensions": dimensions,
                "documentTemplate": TTRPG_DOCUMENT_TEMPLATE,
                "documentTemplateMaxBytes": DOCUMENT_TEMPLATE_MAX_BYTES
            })
        }
    }
}

// ============================================================================
// Index Helpers
// ============================================================================

/// Get all content index names
pub fn all_indexes() -> Vec<&'static str> {
    vec![INDEX_RULES, INDEX_FICTION, INDEX_CHAT, INDEX_DOCUMENTS]
}

/// Select appropriate index based on source type
pub fn select_index_for_source_type(source_type: &str) -> &'static str {
    match source_type.to_lowercase().as_str() {
        "rule" | "rules" | "rulebook" | "mechanics" => INDEX_RULES,
        "fiction" | "lore" | "story" | "narrative" => INDEX_FICTION,
        "chat" | "conversation" | "message" => INDEX_CHAT,
        _ => INDEX_DOCUMENTS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_selection() {
        assert_eq!(select_index_for_source_type("rules"), INDEX_RULES);
        assert_eq!(select_index_for_source_type("fiction"), INDEX_FICTION);
        assert_eq!(select_index_for_source_type("chat"), INDEX_CHAT);
        assert_eq!(select_index_for_source_type("pdf"), INDEX_DOCUMENTS);
    }

    #[test]
    fn test_ollama_dimensions() {
        assert_eq!(ollama_embedding_dimensions("nomic-embed-text"), 768);
        assert_eq!(ollama_embedding_dimensions("mxbai-embed-large"), 1024);
        assert_eq!(ollama_embedding_dimensions("all-minilm"), 384);
        assert_eq!(ollama_embedding_dimensions("unknown-model"), 768);
    }

    #[test]
    fn test_build_embedder_json_openai() {
        let config = EmbedderConfig::OpenAI {
            api_key: "test-key".to_string(),
            model: Some("text-embedding-3-small".to_string()),
            dimensions: Some(1536),
        };

        let json = build_embedder_json(&config);
        assert_eq!(json["source"], "openAi");
        assert_eq!(json["apiKey"], "test-key");
        assert!(json["documentTemplate"].as_str().is_some());
    }

    #[test]
    fn test_build_embedder_json_ollama_rest() {
        let config = EmbedderConfig::OllamaRest {
            host: "http://localhost:11434".to_string(),
            model: "nomic-embed-text".to_string(),
            dimensions: 768,
        };

        let json = build_embedder_json(&config);
        assert_eq!(json["source"], "rest");
        assert_eq!(json["dimensions"], 768);
        assert!(json["url"].as_str().unwrap().contains("/api/embeddings"));
    }
}
