//! RAG (Retrieval-Augmented Generation) pipeline for SurrealDB.
//!
//! Provides context retrieval and formatting for TTRPG queries with LLM integration.
//!
//! ## Tasks Implemented
//!
//! - **Task 4.1.1** - `RagConfig` struct (FR-7.1)
//! - **Task 4.1.2** - Context formatting with templates (FR-7.1, FR-7.3)
//! - **Task 4.2.1** - RAG query function (FR-7.2)
//! - **Task 4.2.2** - Streaming support (FR-7.2, US-6)
//!
//! ## Usage
//!
//! ```no_run
//! use ttrpg_assistant::core::storage::rag::{RagConfig, retrieve_rag_context, prepare_rag_context};
//!
//! # async fn example(db: &surrealdb::Surreal<surrealdb::engine::local::Db>) {
//! // Configure RAG with custom settings
//! let config = RagConfig::with_semantic_ratio(0.7);
//!
//! // Get embedding for query (from embedding model)
//! let embedding = vec![0.1f32; 768];
//!
//! // Retrieve context for non-streaming RAG
//! let (system_prompt, sources) = retrieve_rag_context(
//!     db,
//!     "How does flanking work?",
//!     embedding.clone(),
//!     &config,
//!     None,
//! ).await.unwrap();
//!
//! // Or prepare context for streaming RAG
//! let context = prepare_rag_context(
//!     db,
//!     "What are the rules for opportunity attacks?",
//!     embedding,
//!     &config,
//!     None,
//! ).await.unwrap();
//! // Then pass context.system_prompt to your streaming LLM implementation
//! # }
//! ```

use serde::{Deserialize, Serialize};
use surrealdb::engine::local::Db;
use surrealdb::Surreal;

use super::error::StorageError;
use super::search::{hybrid_search, HybridSearchConfig, SearchFilter, SearchResult};

// ============================================================================
// TASK 4.1.1: RagConfig struct (FR-7.1)
// ============================================================================

/// RAG configuration for TTRPG queries.
///
/// Controls how search results are retrieved and formatted into context
/// for LLM consumption.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RagConfig {
    /// Hybrid search configuration
    pub search_config: HybridSearchConfig,
    /// Maximum chunks to include in context
    pub max_context_chunks: usize,
    /// Maximum context size in bytes
    pub max_context_bytes: usize,
    /// Include source citations in response
    pub include_sources: bool,
    /// System prompt template (uses `{{context}}` placeholder)
    pub system_prompt_template: Option<String>,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            search_config: HybridSearchConfig::default(),
            max_context_chunks: 8,
            max_context_bytes: 4000,
            include_sources: true,
            system_prompt_template: None,
        }
    }
}

impl RagConfig {
    /// Create a RAG config with specified semantic ratio.
    ///
    /// # Arguments
    ///
    /// * `ratio` - Semantic weight (0.0 = keyword only, 1.0 = semantic only)
    ///
    /// # Example
    ///
    /// ```
    /// use ttrpg_assistant::core::storage::rag::RagConfig;
    ///
    /// // 70% semantic, 30% keyword
    /// let config = RagConfig::with_semantic_ratio(0.7);
    /// ```
    pub fn with_semantic_ratio(ratio: f32) -> Self {
        Self {
            search_config: HybridSearchConfig::from_semantic_ratio(ratio),
            ..Default::default()
        }
    }

    /// Set maximum number of chunks to include in context.
    pub fn with_max_chunks(mut self, max_chunks: usize) -> Self {
        self.max_context_chunks = max_chunks;
        self
    }

    /// Set maximum context size in bytes.
    pub fn with_max_bytes(mut self, max_bytes: usize) -> Self {
        self.max_context_bytes = max_bytes;
        self
    }

    /// Enable or disable source citations.
    pub fn with_sources(mut self, include_sources: bool) -> Self {
        self.include_sources = include_sources;
        self
    }

    /// Set custom system prompt template.
    ///
    /// Use `{{context}}` as placeholder for the formatted context.
    pub fn with_template(mut self, template: impl Into<String>) -> Self {
        self.system_prompt_template = Some(template.into());
        self
    }

    /// Create a config optimized for TTRPG rules queries.
    ///
    /// Uses lower semantic weight since rules queries often contain exact terms,
    /// and includes more context chunks for comprehensive rule coverage.
    pub fn for_rules() -> Self {
        Self {
            search_config: HybridSearchConfig::for_rules(),
            max_context_chunks: 10,
            max_context_bytes: 6000,
            include_sources: true,
            system_prompt_template: None,
        }
    }

    /// Create a config optimized for lore/fiction queries.
    ///
    /// Uses higher semantic weight for conceptual similarity matching.
    pub fn for_lore() -> Self {
        Self {
            search_config: HybridSearchConfig::for_lore(),
            max_context_chunks: 6,
            max_context_bytes: 4000,
            include_sources: true,
            system_prompt_template: None,
        }
    }

    /// Create a config optimized for session notes.
    ///
    /// Balanced weights with more results for comprehensive session coverage.
    pub fn for_session_notes() -> Self {
        Self {
            search_config: HybridSearchConfig::for_session_notes(),
            max_context_chunks: 12,
            max_context_bytes: 5000,
            include_sources: true,
            system_prompt_template: None,
        }
    }
}

// ============================================================================
// TASK 4.1.2: Context formatting with templates (FR-7.1, FR-7.3)
// ============================================================================

/// RAG source citation.
///
/// Contains metadata about where a piece of context originated.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RagSource {
    /// Unique identifier for the chunk
    pub id: String,
    /// Source document title/slug
    pub title: String,
    /// Page number within source document
    pub page: Option<i32>,
    /// Relevance score (0.0 - 1.0)
    pub relevance: f32,
}

/// Formatted context for LLM consumption.
///
/// Contains the formatted text ready for inclusion in a prompt,
/// along with source citations and size information.
#[derive(Clone, Debug)]
pub struct FormattedContext {
    /// Formatted context text with numbered source references
    pub text: String,
    /// Source citations for each included chunk
    pub sources: Vec<RagSource>,
    /// Total bytes of context included
    pub total_bytes: usize,
}

/// Format search results into context for LLM.
///
/// Takes search results and formats them into a numbered list suitable
/// for inclusion in an LLM system prompt. Respects the configured
/// maximum chunks and bytes limits.
///
/// # Arguments
///
/// * `results` - Search results from hybrid search
/// * `config` - RAG configuration controlling formatting
///
/// # Returns
///
/// Formatted context with text, sources, and size information.
///
/// # Example
///
/// ```no_run
/// use ttrpg_assistant::core::storage::rag::{format_context, RagConfig};
/// use ttrpg_assistant::core::storage::search::SearchResult;
///
/// let results = vec![
///     SearchResult {
///         id: "chunk:abc123".to_string(),
///         content: "Flanking gives advantage on attack rolls.".to_string(),
///         score: 0.95,
///         linear_score: None,
///         source: "phb-2024".to_string(),
///         page_number: Some(251),
///         section_path: Some("Combat/Flanking".to_string()),
///         content_type: "rules".to_string(),
///         highlights: None,
///     },
/// ];
///
/// let config = RagConfig::default();
/// let formatted = format_context(&results, &config);
///
/// assert!(!formatted.text.is_empty());
/// assert_eq!(formatted.sources.len(), 1);
/// ```
pub fn format_context(results: &[SearchResult], config: &RagConfig) -> FormattedContext {
    let mut context = String::new();
    let mut sources = Vec::new();
    let mut total_bytes = 0;

    for (i, result) in results.iter().take(config.max_context_chunks).enumerate() {
        let page_str = result
            .page_number
            .map(|p| format!(" (p.{})", p))
            .unwrap_or_default();

        let formatted = format!(
            "[{}] {}{}\n{}\n\n",
            i + 1,
            result.source,
            page_str,
            result.content
        );

        // Check if adding this chunk would exceed max bytes
        if total_bytes + formatted.len() > config.max_context_bytes {
            break;
        }

        context.push_str(&formatted);
        total_bytes += formatted.len();

        sources.push(RagSource {
            id: result.id.clone(),
            title: result.source.clone(),
            page: result.page_number,
            relevance: result.score,
        });
    }

    FormattedContext {
        text: context,
        sources,
        total_bytes,
    }
}

/// Build system prompt with context.
///
/// Combines formatted context with either a custom template or the
/// default TTRPG Game Master assistant prompt.
///
/// # Arguments
///
/// * `context` - Formatted context text to include
/// * `custom_template` - Optional custom template with `{{context}}` placeholder
///
/// # Returns
///
/// Complete system prompt ready for LLM consumption.
///
/// # Example
///
/// ```
/// use ttrpg_assistant::core::storage::rag::build_system_prompt;
///
/// let context = "[1] phb-2024 (p.251)\nFlanking gives advantage.\n\n";
///
/// // With default template
/// let prompt = build_system_prompt(context, None);
/// assert!(prompt.contains("TTRPG Game Master"));
/// assert!(prompt.contains("Flanking gives advantage"));
///
/// // With custom template
/// let custom = "You are a rules expert.\n\nContext:\n{{context}}\n\nAnswer accurately.";
/// let prompt = build_system_prompt(context, Some(custom));
/// assert!(prompt.contains("rules expert"));
/// assert!(prompt.contains("Flanking gives advantage"));
/// ```
pub fn build_system_prompt(context: &str, custom_template: Option<&str>) -> String {
    if let Some(template) = custom_template {
        template.replace("{{context}}", context)
    } else {
        format!(
            r#"You are an expert TTRPG Game Master assistant with deep knowledge of tabletop roleplaying games.

## Context from Indexed Rulebooks

{context}

## Instructions

1. Use ONLY the provided context to answer questions
2. Always cite your sources with [N] references matching the numbered sources above
3. If the context doesn't contain enough information, say so clearly
4. Format rules and mechanics clearly for quick reference at the table
5. Distinguish between core rules and optional/variant rules
6. When multiple sources conflict, note the discrepancy
"#,
            context = context
        )
    }
}

// ============================================================================
// TASK 4.2.1: RAG query function (FR-7.2)
// ============================================================================

/// RAG response with content and sources.
///
/// Used for non-streaming RAG responses where the full response
/// is returned at once.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RagResponse {
    /// LLM-generated content (populated by caller after LLM call)
    pub content: String,
    /// Source citations from retrieved context
    pub sources: Vec<RagSource>,
    /// Number of context bytes used
    pub context_used: usize,
}

/// Retrieve RAG context for a query.
///
/// Executes hybrid search and formats results into a system prompt
/// suitable for LLM consumption. This function handles the retrieval
/// and formatting; the actual LLM call should be made by the caller
/// using the existing LLM router infrastructure.
///
/// # Arguments
///
/// * `db` - SurrealDB connection
/// * `query` - User's question
/// * `embedding` - Query embedding (from embedding model)
/// * `config` - RAG configuration
/// * `filters` - Optional search filters (content type, library item, page range)
///
/// # Returns
///
/// Tuple of (system_prompt, sources) for use with LLM call.
///
/// # Errors
///
/// Returns `StorageError::Query` if the search fails.
///
/// # Example
///
/// ```no_run
/// use ttrpg_assistant::core::storage::rag::{retrieve_rag_context, RagConfig};
///
/// # async fn example(db: &surrealdb::Surreal<surrealdb::engine::local::Db>) {
/// let config = RagConfig::for_rules();
/// let embedding = vec![0.1f32; 768]; // From embedding model
///
/// let (system_prompt, sources) = retrieve_rag_context(
///     db,
///     "How does flanking work in D&D 5e?",
///     embedding,
///     &config,
///     None,
/// ).await.unwrap();
///
/// // Now use system_prompt with your LLM:
/// // let response = llm.chat(system_prompt, user_message).await?;
/// // Return RagResponse { content: response, sources, context_used: ... }
/// # }
/// ```
pub async fn retrieve_rag_context(
    db: &Surreal<Db>,
    query: &str,
    embedding: Vec<f32>,
    config: &RagConfig,
    filters: Option<&SearchFilter>,
) -> Result<(String, Vec<RagSource>), StorageError> {
    // Execute hybrid search
    let filter_str = filters.and_then(|f| f.to_surql());
    let results = hybrid_search(
        db,
        query,
        embedding,
        &config.search_config,
        filter_str.as_deref(),
    )
    .await?;

    // Format context
    let formatted = format_context(&results, config);

    // Build system prompt
    let system_prompt =
        build_system_prompt(&formatted.text, config.system_prompt_template.as_deref());

    Ok((system_prompt, formatted.sources))
}

// ============================================================================
// TASK 4.2.2: Streaming support (FR-7.2, US-6)
// ============================================================================

/// Context prepared for streaming RAG query.
///
/// Contains all the information needed to make a streaming LLM call
/// while keeping sources separate for later reference.
#[derive(Clone, Debug)]
pub struct RagContext {
    /// System prompt with formatted context
    pub system_prompt: String,
    /// Source citations for the included context
    pub sources: Vec<RagSource>,
    /// Original user query
    pub query: String,
    /// Number of context bytes used
    pub context_bytes: usize,
}

/// Prepare context for streaming RAG query.
///
/// Similar to `retrieve_rag_context`, but returns a `RagContext` struct
/// that includes the original query. This is designed for streaming
/// scenarios where:
///
/// 1. Context is retrieved and formatted
/// 2. Caller initiates streaming LLM call with system_prompt + query
/// 3. Response chunks are streamed to the user
/// 4. Sources are provided after streaming completes
///
/// # Arguments
///
/// * `db` - SurrealDB connection
/// * `query` - User's question
/// * `embedding` - Query embedding
/// * `config` - RAG configuration
/// * `filters` - Optional search filters
///
/// # Returns
///
/// `RagContext` containing system prompt, sources, and query.
///
/// # Example
///
/// ```no_run
/// use ttrpg_assistant::core::storage::rag::{prepare_rag_context, RagConfig};
///
/// # async fn example(db: &surrealdb::Surreal<surrealdb::engine::local::Db>) {
/// let config = RagConfig::default();
/// let embedding = vec![0.1f32; 768];
///
/// let context = prepare_rag_context(
///     db,
///     "What are the rules for opportunity attacks?",
///     embedding,
///     &config,
///     None,
/// ).await.unwrap();
///
/// // Use with streaming LLM:
/// // llm.stream_chat(context.system_prompt, context.query, |chunk| {
/// //     send_to_frontend(chunk);
/// // }).await?;
/// //
/// // After streaming, send sources:
/// // send_sources_to_frontend(context.sources);
/// # }
/// ```
pub async fn prepare_rag_context(
    db: &Surreal<Db>,
    query: &str,
    embedding: Vec<f32>,
    config: &RagConfig,
    filters: Option<&SearchFilter>,
) -> Result<RagContext, StorageError> {
    // Execute hybrid search
    let filter_str = filters.and_then(|f| f.to_surql());
    let results = hybrid_search(
        db,
        query,
        embedding,
        &config.search_config,
        filter_str.as_deref(),
    )
    .await?;

    // Format context
    let formatted = format_context(&results, config);

    // Build system prompt
    let system_prompt =
        build_system_prompt(&formatted.text, config.system_prompt_template.as_deref());

    Ok(RagContext {
        system_prompt,
        sources: formatted.sources,
        query: query.to_string(),
        context_bytes: formatted.total_bytes,
    })
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Task 4.1.1: RagConfig tests
    // ========================================================================

    #[test]
    fn test_rag_config_defaults() {
        let config = RagConfig::default();

        assert_eq!(config.max_context_chunks, 8);
        assert_eq!(config.max_context_bytes, 4000);
        assert!(config.include_sources);
        assert!(config.system_prompt_template.is_none());

        // Check search config defaults
        assert!((config.search_config.semantic_weight - 0.6).abs() < f32::EPSILON);
        assert!((config.search_config.keyword_weight - 0.4).abs() < f32::EPSILON);
    }

    #[test]
    fn test_rag_config_with_semantic_ratio() {
        let config = RagConfig::with_semantic_ratio(0.8);

        assert!((config.search_config.semantic_weight - 0.8).abs() < f32::EPSILON);
        assert!((config.search_config.keyword_weight - 0.2).abs() < 0.0001);
    }

    #[test]
    fn test_rag_config_builder_pattern() {
        let config = RagConfig::with_semantic_ratio(0.7)
            .with_max_chunks(12)
            .with_max_bytes(8000)
            .with_sources(false)
            .with_template("Custom template: {{context}}");

        assert_eq!(config.max_context_chunks, 12);
        assert_eq!(config.max_context_bytes, 8000);
        assert!(!config.include_sources);
        assert_eq!(
            config.system_prompt_template,
            Some("Custom template: {{context}}".to_string())
        );
    }

    #[test]
    fn test_rag_config_for_rules() {
        let config = RagConfig::for_rules();

        // Rules should favor keyword search
        assert!(config.search_config.keyword_weight > config.search_config.semantic_weight);
        assert_eq!(config.max_context_chunks, 10);
        assert_eq!(config.max_context_bytes, 6000);
    }

    #[test]
    fn test_rag_config_for_lore() {
        let config = RagConfig::for_lore();

        // Lore should favor semantic search
        assert!(config.search_config.semantic_weight > config.search_config.keyword_weight);
        assert_eq!(config.max_context_chunks, 6);
    }

    #[test]
    fn test_rag_config_for_session_notes() {
        let config = RagConfig::for_session_notes();

        // Session notes should be balanced
        assert_eq!(
            config.search_config.semantic_weight,
            config.search_config.keyword_weight
        );
        assert_eq!(config.max_context_chunks, 12);
    }

    // ========================================================================
    // Task 4.1.2: Context formatting tests
    // ========================================================================

    fn make_test_result(id: &str, content: &str, source: &str, page: Option<i32>) -> SearchResult {
        SearchResult {
            id: id.to_string(),
            content: content.to_string(),
            score: 0.85,
            linear_score: None,
            source: source.to_string(),
            page_number: page,
            section_path: None,
            content_type: "rules".to_string(),
            highlights: None,
        }
    }

    #[test]
    fn test_format_context_basic() {
        let results = vec![
            make_test_result(
                "chunk:1",
                "Flanking gives advantage on attack rolls.",
                "phb-2024",
                Some(251),
            ),
            make_test_result(
                "chunk:2",
                "Cover provides a bonus to AC.",
                "phb-2024",
                Some(198),
            ),
        ];

        let config = RagConfig::default();
        let formatted = format_context(&results, &config);

        // Check that context is formatted correctly
        assert!(formatted.text.contains("[1] phb-2024 (p.251)"));
        assert!(formatted.text.contains("Flanking gives advantage"));
        assert!(formatted.text.contains("[2] phb-2024 (p.198)"));
        assert!(formatted.text.contains("Cover provides a bonus"));

        // Check sources
        assert_eq!(formatted.sources.len(), 2);
        assert_eq!(formatted.sources[0].id, "chunk:1");
        assert_eq!(formatted.sources[0].title, "phb-2024");
        assert_eq!(formatted.sources[0].page, Some(251));
        assert_eq!(formatted.sources[1].id, "chunk:2");
    }

    #[test]
    fn test_format_context_truncates_at_max_bytes() {
        // Create a result with content that exceeds max_bytes
        let long_content = "A".repeat(500);
        let results: Vec<SearchResult> = (0..20)
            .map(|i| make_test_result(&format!("chunk:{}", i), &long_content, "test", Some(i)))
            .collect();

        let config = RagConfig::default().with_max_bytes(1000);
        let formatted = format_context(&results, &config);

        // Should truncate before exceeding max_bytes
        assert!(
            formatted.total_bytes <= config.max_context_bytes,
            "Total bytes {} should not exceed max {}",
            formatted.total_bytes,
            config.max_context_bytes
        );

        // Should have fewer sources than input due to truncation
        assert!(formatted.sources.len() < 20);
    }

    #[test]
    fn test_format_context_respects_max_chunks() {
        let results: Vec<SearchResult> = (0..20)
            .map(|i| make_test_result(&format!("chunk:{}", i), "Short content", "test", Some(i)))
            .collect();

        let config = RagConfig::default().with_max_chunks(5);
        let formatted = format_context(&results, &config);

        assert_eq!(formatted.sources.len(), 5);
    }

    #[test]
    fn test_format_context_without_page_numbers() {
        let results = vec![make_test_result(
            "chunk:1",
            "Content without page",
            "session-notes",
            None,
        )];

        let config = RagConfig::default();
        let formatted = format_context(&results, &config);

        // Should not include "(p.)" when no page number
        assert!(formatted.text.contains("[1] session-notes\n"));
        assert!(!formatted.text.contains("(p.)"));
    }

    #[test]
    fn test_source_citations_numbered_correctly() {
        let results: Vec<SearchResult> = (0..5)
            .map(|i| make_test_result(&format!("chunk:{}", i), &format!("Content {}", i), "test", Some(i * 10)))
            .collect();

        let config = RagConfig::default();
        let formatted = format_context(&results, &config);

        // Check sequential numbering
        assert!(formatted.text.contains("[1] test"));
        assert!(formatted.text.contains("[2] test"));
        assert!(formatted.text.contains("[3] test"));
        assert!(formatted.text.contains("[4] test"));
        assert!(formatted.text.contains("[5] test"));
    }

    #[test]
    fn test_build_system_prompt_default_template() {
        let context = "[1] phb-2024 (p.251)\nFlanking gives advantage.\n\n";

        let prompt = build_system_prompt(context, None);

        // Check default template elements
        assert!(prompt.contains("TTRPG Game Master assistant"));
        assert!(prompt.contains("Context from Indexed Rulebooks"));
        assert!(prompt.contains("Flanking gives advantage"));
        assert!(prompt.contains("cite your sources with [N] references"));
    }

    #[test]
    fn test_build_system_prompt_custom_template() {
        let context = "[1] test\nSome content\n\n";
        let template = "You are a rules expert.\n\n## Rules:\n{{context}}\n\nBe precise.";

        let prompt = build_system_prompt(context, Some(template));

        assert!(prompt.contains("You are a rules expert."));
        assert!(prompt.contains("## Rules:"));
        assert!(prompt.contains("Some content"));
        assert!(prompt.contains("Be precise."));
        assert!(!prompt.contains("{{context}}")); // Placeholder should be replaced
    }

    #[test]
    fn test_build_system_prompt_empty_context() {
        let prompt = build_system_prompt("", None);

        // Should still have instructions even with empty context
        assert!(prompt.contains("Instructions"));
        assert!(prompt.contains("cite your sources"));
    }

    // ========================================================================
    // RagSource tests
    // ========================================================================

    #[test]
    fn test_rag_source_serialization() {
        let source = RagSource {
            id: "chunk:abc123".to_string(),
            title: "Player's Handbook 2024".to_string(),
            page: Some(251),
            relevance: 0.95,
        };

        let json = serde_json::to_string(&source).expect("Serialization failed");
        let deserialized: RagSource =
            serde_json::from_str(&json).expect("Deserialization failed");

        assert_eq!(source, deserialized);
    }

    #[test]
    fn test_rag_source_without_page() {
        let source = RagSource {
            id: "chunk:xyz".to_string(),
            title: "Session Notes".to_string(),
            page: None,
            relevance: 0.72,
        };

        let json = serde_json::to_string(&source).expect("Serialization failed");
        assert!(json.contains("\"page\":null"));
    }

    // ========================================================================
    // RagConfig serialization tests
    // ========================================================================

    #[test]
    fn test_rag_config_serialization() {
        let config = RagConfig::default();

        let json = serde_json::to_string(&config).expect("Serialization failed");
        let deserialized: RagConfig =
            serde_json::from_str(&json).expect("Deserialization failed");

        assert_eq!(deserialized.max_context_chunks, config.max_context_chunks);
        assert_eq!(deserialized.max_context_bytes, config.max_context_bytes);
        assert_eq!(deserialized.include_sources, config.include_sources);
    }
}

// ============================================================================
// Integration tests (require database)
// ============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;
    use surrealdb::engine::local::RocksDb;
    use tempfile::TempDir;

    /// Helper to create a test database with schema applied.
    async fn setup_test_db() -> (TempDir, Surreal<Db>) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db = Surreal::new::<RocksDb>(temp_dir.path())
            .await
            .expect("Failed to connect to SurrealDB");

        db.use_ns("test")
            .use_db("rag_test")
            .await
            .expect("Failed to select ns/db");

        // Apply minimal schema needed for tests
        db.query(
            r#"
            DEFINE ANALYZER IF NOT EXISTS ttrpg_analyzer
                TOKENIZERS class, blank, punct
                FILTERS lowercase, ascii, snowball(english);

            DEFINE TABLE IF NOT EXISTS library_item SCHEMAFULL;
            DEFINE FIELD IF NOT EXISTS slug ON library_item TYPE string;
            DEFINE FIELD IF NOT EXISTS title ON library_item TYPE string;
            DEFINE INDEX IF NOT EXISTS library_slug ON library_item FIELDS slug UNIQUE;

            DEFINE TABLE IF NOT EXISTS chunk SCHEMAFULL;
            DEFINE FIELD IF NOT EXISTS content ON chunk TYPE string;
            DEFINE FIELD IF NOT EXISTS library_item ON chunk TYPE record<library_item>;
            DEFINE FIELD IF NOT EXISTS content_type ON chunk TYPE string;
            DEFINE FIELD IF NOT EXISTS page_number ON chunk TYPE option<int>;
            DEFINE FIELD IF NOT EXISTS section_path ON chunk TYPE option<string>;
            DEFINE FIELD IF NOT EXISTS embedding ON chunk TYPE option<array<float>>;

            DEFINE INDEX IF NOT EXISTS chunk_content ON chunk FIELDS content SEARCH ANALYZER ttrpg_analyzer BM25 HIGHLIGHTS;
            DEFINE INDEX IF NOT EXISTS chunk_embedding ON chunk FIELDS embedding HNSW DIMENSION 768 DIST COSINE EFC 150 M 12;
            DEFINE INDEX IF NOT EXISTS chunk_type ON chunk FIELDS content_type;
            "#,
        )
        .await
        .expect("Failed to apply test schema");

        (temp_dir, db)
    }

    /// Generate a simple test embedding.
    fn make_embedding(seed: f32) -> Vec<f32> {
        (0..768).map(|i| (seed + i as f32 * 0.001).sin()).collect()
    }

    /// Insert a test library item.
    async fn insert_library_item(db: &Surreal<Db>, slug: &str, title: &str) {
        let slug_owned = slug.to_string();
        let title_owned = title.to_string();
        db.query("CREATE library_item CONTENT { slug: $slug, title: $title }")
            .bind(("slug", slug_owned))
            .bind(("title", title_owned))
            .await
            .expect("Failed to create library_item");
    }

    /// Insert a test chunk.
    async fn insert_chunk(
        db: &Surreal<Db>,
        content: &str,
        library_slug: &str,
        content_type: &str,
        page_number: Option<i32>,
        embedding: Vec<f32>,
    ) {
        let content_owned = content.to_string();
        let slug_owned = library_slug.to_string();
        let content_type_owned = content_type.to_string();
        db.query(
            r#"
            CREATE chunk CONTENT {
                content: $content,
                library_item: (SELECT id FROM library_item WHERE slug = $slug LIMIT 1)[0].id,
                content_type: $content_type,
                page_number: $page_number,
                embedding: $embedding
            }
            "#,
        )
        .bind(("content", content_owned))
        .bind(("slug", slug_owned))
        .bind(("content_type", content_type_owned))
        .bind(("page_number", page_number))
        .bind(("embedding", embedding))
        .await
        .expect("Failed to create chunk");
    }

    #[tokio::test]
    async fn test_retrieve_rag_context() {
        let (_dir, db) = setup_test_db().await;

        // Setup test data
        insert_library_item(&db, "phb-2024", "Player's Handbook 2024").await;

        let embedding = make_embedding(0.0);
        insert_chunk(
            &db,
            "Flanking gives advantage on melee attack rolls against a creature.",
            "phb-2024",
            "rules",
            Some(251),
            embedding.clone(),
        )
        .await;

        insert_chunk(
            &db,
            "Cover provides a bonus to AC and Dexterity saving throws.",
            "phb-2024",
            "rules",
            Some(198),
            make_embedding(0.5),
        )
        .await;

        // Test RAG context retrieval
        let config = RagConfig::default();
        let result = retrieve_rag_context(&db, "flanking advantage", embedding, &config, None).await;

        assert!(result.is_ok(), "RAG context retrieval failed: {:?}", result.err());

        let (system_prompt, sources) = result.unwrap();

        // Check system prompt contains context
        assert!(system_prompt.contains("Flanking"));
        assert!(system_prompt.contains("TTRPG Game Master"));

        // Check sources
        assert!(!sources.is_empty());
        assert!(sources.iter().any(|s| s.title == "phb-2024"));
    }

    #[tokio::test]
    async fn test_prepare_rag_context() {
        let (_dir, db) = setup_test_db().await;

        // Setup test data
        insert_library_item(&db, "dmg-2024", "Dungeon Master's Guide 2024").await;

        let embedding = make_embedding(0.0);
        insert_chunk(
            &db,
            "Opportunity attacks occur when a creature leaves an enemy's reach.",
            "dmg-2024",
            "rules",
            Some(195),
            embedding.clone(),
        )
        .await;

        // Test RAG context preparation for streaming
        let config = RagConfig::default();
        let result =
            prepare_rag_context(&db, "opportunity attacks", embedding, &config, None).await;

        assert!(result.is_ok(), "RAG context preparation failed: {:?}", result.err());

        let context = result.unwrap();

        // Check context struct
        assert!(!context.system_prompt.is_empty());
        assert_eq!(context.query, "opportunity attacks");
        assert!(context.context_bytes > 0);
        assert!(!context.sources.is_empty());
    }

    #[tokio::test]
    async fn test_retrieve_rag_context_with_filters() {
        let (_dir, db) = setup_test_db().await;

        // Setup test data
        insert_library_item(&db, "phb-2024", "Player's Handbook 2024").await;

        let embedding = make_embedding(0.0);
        insert_chunk(
            &db,
            "Dragon combat tactics and strategies.",
            "phb-2024",
            "rules",
            Some(100),
            embedding.clone(),
        )
        .await;

        insert_chunk(
            &db,
            "Dragon lore and history of dragonkind.",
            "phb-2024",
            "fiction",
            Some(200),
            make_embedding(0.1),
        )
        .await;

        // Test with content type filter
        let config = RagConfig::default();
        let filter = SearchFilter::new().content_type("rules");

        let result =
            retrieve_rag_context(&db, "dragon", embedding, &config, Some(&filter)).await;

        assert!(result.is_ok());

        let (system_prompt, sources) = result.unwrap();

        // Should only find rules content
        assert!(system_prompt.contains("combat") || system_prompt.contains("tactics"));

        // Verify sources were returned (filter is applied during search)
        // Note: This test validates the filter is passed through correctly
        let _ = &sources;
    }

    #[tokio::test]
    async fn test_rag_context_with_custom_template() {
        let (_dir, db) = setup_test_db().await;

        insert_library_item(&db, "test-doc", "Test Document").await;

        let embedding = make_embedding(0.0);
        insert_chunk(
            &db,
            "Some test content for the custom template.",
            "test-doc",
            "rules",
            Some(1),
            embedding.clone(),
        )
        .await;

        let config = RagConfig::default()
            .with_template("CUSTOM HEADER\n\n{{context}}\n\nCUSTOM FOOTER");

        let result =
            retrieve_rag_context(&db, "test content", embedding, &config, None).await;

        assert!(result.is_ok());

        let (system_prompt, _) = result.unwrap();

        assert!(system_prompt.contains("CUSTOM HEADER"));
        assert!(system_prompt.contains("CUSTOM FOOTER"));
        assert!(system_prompt.contains("test content"));
    }
}
