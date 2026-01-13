//! Meilisearch Client Module
//!
//! Provides a unified search interface using Meilisearch with specialized indexes
//! for different content types (rules, fiction, chat, documents).

use meilisearch_sdk::client::Client;
use meilisearch_sdk::indexes::Index;
use meilisearch_sdk::settings::Settings;
use meilisearch_sdk::search::SearchResults;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

// ============================================================================
// Constants
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

/// Default timeout for short operations (metadata updates, single deletes)
pub const TASK_TIMEOUT_SHORT_SECS: u64 = 30;
/// Default timeout for long operations (bulk ingestion, index creation)
pub const TASK_TIMEOUT_LONG_SECS: u64 = 600; // 10 minutes

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("Meilisearch error: {0}")]
    MeilisearchError(String),

    #[error("Index not found: {0}")]
    IndexNotFound(String),

    #[error("Document not found: {0}")]
    DocumentNotFound(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

impl From<meilisearch_sdk::errors::Error> for SearchError {
    fn from(e: meilisearch_sdk::errors::Error) -> Self {
        SearchError::MeilisearchError(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, SearchError>;

// ============================================================================
// Document Types
// ============================================================================

/// A searchable document chunk
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchDocument {
    /// Unique document ID
    pub id: String,
    /// Text content
    pub content: String,
    /// Source file or origin (file path)
    pub source: String,
    /// Source type for categorization (rule, fiction, chat, document)
    #[serde(default)]
    pub source_type: String,
    /// Page number if from PDF
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_number: Option<u32>,
    /// Chunk index within document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_index: Option<u32>,
    /// Campaign ID if associated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub campaign_id: Option<String>,
    /// Session ID if from chat
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Creation timestamp
    pub created_at: String,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,

    // =========================================================================
    // TTRPG-Specific Embedding Metadata
    // These fields are included in the documentTemplate for semantic search
    // =========================================================================

    /// Human-readable book/document title (e.g., "Delta Green: Handler's Guide")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub book_title: Option<String>,

    /// Game system display name (e.g., "Delta Green", "D&D 5th Edition")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_system: Option<String>,

    /// Game system machine ID (e.g., "delta_green", "dnd5e")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_system_id: Option<String>,

    /// Content category: rulebook, adventure, setting, supplement, bestiary, quickstart
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_category: Option<String>,

    /// Section/chapter title (e.g., "Chapter 3: Combat", "Appendix A: Monsters")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section_title: Option<String>,

    /// Genre/theme (e.g., "cosmic horror", "fantasy", "sci-fi")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,

    /// Publisher name (e.g., "Arc Dream Publishing", "Wizards of the Coast")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,

    // =========================================================================
    // Enhanced Metadata (from MDMAI patterns)
    // =========================================================================

    /// Chunk type for content classification (text, stat_block, table, spell, monster, rule, narrative)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunk_type: Option<String>,

    /// Chapter title (top-level section from TOC)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chapter_title: Option<String>,

    /// Subsection title (nested within section)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subsection_title: Option<String>,

    /// Full section hierarchy path (e.g., "Chapter 1 > Monsters > Goblins")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section_path: Option<String>,

    /// Mechanic type for rules content (skill_check, combat, damage, healing, sanity, etc.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mechanic_type: Option<String>,

    /// Extracted semantic keywords for embedding boost
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub semantic_keywords: Vec<String>,
}

/// A search result with score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub document: SearchDocument,
    pub score: f32,
    pub index: String,
}

/// Federated search results from multiple indexes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedResults {
    pub results: Vec<SearchResult>,
    pub total_hits: usize,
    pub processing_time_ms: u64,
}

/// Library document metadata - stored in Meilisearch for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryDocumentMetadata {
    /// Unique document ID
    pub id: String,
    /// Document name (file name without path)
    pub name: String,
    /// Source type (pdf, epub, mobi, docx, txt)
    pub source_type: String,
    /// Original file path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    /// Number of pages in the document
    pub page_count: u32,
    /// Number of chunks indexed
    pub chunk_count: u32,
    /// Total characters extracted
    pub character_count: u64,
    /// Index where content chunks are stored (rules, fiction, documents)
    pub content_index: String,
    /// Processing status (pending, processing, ready, error)
    pub status: String,
    /// Error message if status is error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// Timestamp when ingested
    pub ingested_at: String,
}

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
    Ollama {
        url: String,
        model: String,
    },
    /// HuggingFace embeddings
    #[serde(rename = "huggingFace")]
    HuggingFace {
        model: String,
    },
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

// ============================================================================
// Search Client
// ============================================================================

pub struct SearchClient {
    client: Client,
    host: String,
    api_key: Option<String>,
}

impl SearchClient {
    pub fn new(host: &str, api_key: Option<&str>) -> Self {
        Self {
            client: Client::new(host, api_key).expect("Failed to create Meilisearch client"),
            host: host.to_string(),
            api_key: api_key.map(|s| s.to_string()),
        }
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    /// Get the underlying Meilisearch client
    pub fn get_client(&self) -> &Client {
        &self.client
    }

    /// Check if Meilisearch is healthy
    pub async fn health_check(&self) -> bool {
        // Use raw reqwest to avoid SDK parsing errors if version mismatch
        let url = format!("{}/health", self.host);
        let client = reqwest::Client::new();
        match client.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Wait for Meilisearch to become healthy
    pub async fn wait_for_health(&self, timeout_secs: u64) -> bool {
        let start = std::time::Instant::now();
        let duration = std::time::Duration::from_secs(timeout_secs);
        while start.elapsed() < duration {
            if self.health_check().await {
                return true;
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        false
    }

    /// Get an index by name
    pub fn index(&self, name: &str) -> Index {
        self.client.index(name)
    }

    /// Create or get an index
    pub async fn ensure_index(&self, name: &str, primary_key: Option<&str>) -> Result<Index> {
        // Try to get existing index first
        match self.client.get_index(name).await {
            Ok(idx) => Ok(idx),
            Err(_) => {
                // Create new index
                let task = self.client
                    .create_index(name, primary_key)
                    .await?;
                task.wait_for_completion(
                    &self.client,
                    Some(std::time::Duration::from_millis(100)),
                    Some(std::time::Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)),
                ).await?;
                Ok(self.client.index(name))
            }
        }
    }

    /// Initialize all specialized indexes with appropriate settings
    pub async fn initialize_indexes(&self) -> Result<()> {
        // Enable experimental features (vectorStore) required for hybrid search
        let url = format!("{}/experimental-features", self.host);
        let client = reqwest::Client::new();
        let mut request = client.patch(&url)
            .json(&serde_json::json!({
                "vectorStore": true,
                "scoreDetails": true
            }));

        if let Some(key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        if let Err(e) = request.send().await {
            log::warn!("Failed to enable experimental features: {}", e);
        } else {
            log::info!("Enabled Meilisearch experimental features (vectorStore, scoreDetails)");
        }

        // Create all content indexes
        for index_name in [INDEX_RULES, INDEX_FICTION, INDEX_CHAT, INDEX_DOCUMENTS] {
            self.ensure_index(index_name, Some("id")).await?;
        }

        // Create library metadata index
        self.ensure_index(INDEX_LIBRARY_METADATA, Some("id")).await?;

        // Configure settings for content indexes
        let base_settings = Settings::new()
            .with_searchable_attributes(["content", "source", "metadata"])
            // .with_filterable_attributes(["source", "source_type", "campaign_id", "session_id", "created_at"])
            .with_sortable_attributes(["created_at"]);

        // Apply settings to content indexes
        for index_name in [INDEX_RULES, INDEX_FICTION, INDEX_CHAT, INDEX_DOCUMENTS] {
            let index = self.client.index(index_name);
            let task = index.set_settings(&base_settings).await?;
            task.wait_for_completion(
                &self.client,
                Some(std::time::Duration::from_millis(100)),
                Some(std::time::Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)),
            ).await?;
        }

        // Configure library metadata index settings
        let library_settings = Settings::new()
            .with_searchable_attributes(["name", "source_type", "file_path"])
            .with_filterable_attributes(["source_type", "status", "content_index", "ingested_at"])
            .with_sortable_attributes(["ingested_at", "name", "page_count", "chunk_count"]);

        let library_index = self.client.index(INDEX_LIBRARY_METADATA);
        let task = library_index.set_settings(&library_settings).await?;
        task.wait_for_completion(
            &self.client,
            Some(std::time::Duration::from_millis(100)),
            Some(std::time::Duration::from_secs(30)),
        ).await?;

        log::info!("Initialized Meilisearch indexes: rules, fiction, chat, documents, library_metadata");
        Ok(())
    }

    /// Configure an embedder for semantic search on an index
    pub async fn configure_embedder(
        &self,
        index_name: &str,
        embedder_name: &str,
        config: &EmbedderConfig,
    ) -> Result<()> {
        // Rich document template for TTRPG content semantic embedding
        // Uses Liquid templating with conditional fields for context-aware embedding
        // Based on MDMAI patterns for optimal embedding quality
        //
        // Example output:
        // "[STAT_BLOCK] [Delta Green] Agent's Handbook (rulebook, p.96) - Equipment > Weapons [combat] [cosmic horror]
        //  Type: stat_block
        //  Topics: firearms, damage, concealment
        //  The standard handgun is a semi-automatic pistol..."
        let document_template = r#"{% if doc.chunk_type and doc.chunk_type != "text" and doc.chunk_type != "narrative" %}[{{ doc.chunk_type | upcase }}] {% endif %}{% if doc.game_system %}[{{ doc.game_system }}] {% endif %}{% if doc.book_title %}{{ doc.book_title }}{% else %}{{ doc.source }}{% endif %}{% if doc.content_category %} ({{ doc.content_category }}{% if doc.page_number %}, p.{{ doc.page_number }}{% endif %}){% elsif doc.page_number %} (p.{{ doc.page_number }}){% endif %}{% if doc.section_path %} - {{ doc.section_path }}{% elsif doc.chapter_title %} - {{ doc.chapter_title }}{% if doc.section_title %} > {{ doc.section_title }}{% endif %}{% if doc.subsection_title %} > {{ doc.subsection_title }}{% endif %}{% elsif doc.section_title %} - {{ doc.section_title }}{% endif %}{% if doc.mechanic_type %} [{{ doc.mechanic_type }}]{% endif %}{% if doc.genre %} [{{ doc.genre }}]{% endif %}
Type: {{ doc.chunk_type | default: "text" }}
{% if doc.semantic_keywords.size > 0 %}Topics: {{ doc.semantic_keywords | join: ", " }}
{% endif %}{{ doc.content | truncatewords: 300 }}"#;

        // Build embedder settings as JSON
        let embedder_json = match config {
            EmbedderConfig::OpenAI { api_key, model, dimensions } => {
                serde_json::json!({
                    "source": "openAi",
                    "apiKey": api_key,
                    "model": model.clone().unwrap_or_else(|| "text-embedding-3-small".to_string()),
                    "dimensions": dimensions.unwrap_or(1536),
                    "documentTemplate": document_template,
                    "documentTemplateMaxBytes": 4000
                })
            }
            EmbedderConfig::Ollama { url, model } => {
                serde_json::json!({
                    "source": "ollama",
                    "url": url,
                    "model": model,
                    "documentTemplate": document_template,
                    "documentTemplateMaxBytes": 4000
                })
            }
            EmbedderConfig::HuggingFace { model } => {
                serde_json::json!({
                    "source": "huggingFace",
                    "model": model,
                    "documentTemplate": document_template,
                    "documentTemplateMaxBytes": 4000
                })
            }
            EmbedderConfig::OllamaRest { host, model, dimensions } => {
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
                    "documentTemplate": document_template,
                    "documentTemplateMaxBytes": 4000
                })
            }
        };

        // Use PATCH to update embedders setting
        let url = format!("{}/indexes/{}/settings/embedders", self.host, index_name);
        let client = reqwest::Client::new();

        let mut request = client.patch(&url)
            .json(&serde_json::json!({ embedder_name: embedder_json }));

        if let Some(key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response = request.send().await
            .map_err(|e| SearchError::ConfigError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SearchError::ConfigError(format!(
                "Failed to configure embedder: {}", error_text
            )));
        }

        log::info!("Configured embedder '{}' for index '{}'", embedder_name, index_name);
        Ok(())
    }

    /// Configure Ollama REST embedder on all content indexes
    ///
    /// This sets up AI-powered semantic search using Ollama's embedding API.
    /// The embedder is named "ollama" and uses the REST source type for compatibility.
    pub async fn setup_ollama_embeddings(&self, host: &str, model: &str) -> Result<Vec<String>> {
        let dimensions = ollama_embedding_dimensions(model);
        let config = EmbedderConfig::OllamaRest {
            host: host.to_string(),
            model: model.to_string(),
            dimensions,
        };

        let mut configured = Vec::new();
        let content_indexes = Self::all_indexes();

        for index_name in content_indexes {
            match self.configure_embedder(index_name, "ollama", &config).await {
                Ok(_) => {
                    log::info!("Configured Ollama embedder on index '{}' with model '{}'", index_name, model);
                    configured.push(index_name.to_string());
                }
                Err(e) => {
                    log::warn!("Failed to configure embedder on '{}': {}", index_name, e);
                }
            }
        }

        if configured.is_empty() {
            return Err(SearchError::ConfigError("Failed to configure any indexes".to_string()));
        }

        Ok(configured)
    }

    /// Get current embedder configuration for an index
    pub async fn get_embedder_settings(&self, index_name: &str) -> Result<Option<serde_json::Value>> {
        let url = format!("{}/indexes/{}/settings/embedders", self.host, index_name);
        let client = reqwest::Client::new();

        let mut request = client.get(&url);
        if let Some(key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response = request.send().await
            .map_err(|e| SearchError::ConfigError(e.to_string()))?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let settings: serde_json::Value = response.json().await
            .map_err(|e| SearchError::ConfigError(e.to_string()))?;

        Ok(Some(settings))
    }

    // ========================================================================
    // Document Operations
    // ========================================================================

    /// Add documents to an index
    pub async fn add_documents(&self, index_name: &str, documents: Vec<SearchDocument>) -> Result<()> {
        if documents.is_empty() {
            log::warn!("add_documents called with empty document list for index '{}' - nothing indexed", index_name);
            return Ok(());
        }

        let index = self.client.index(index_name);
        let task = index.add_documents(&documents, Some("id")).await?;

        // Wait with explicit timeout (10 minutes) and polling interval (100ms)
        task.wait_for_completion(
            &self.client,
            Some(std::time::Duration::from_millis(100)),
            Some(std::time::Duration::from_secs(TASK_TIMEOUT_LONG_SECS)),
        ).await?;

        log::info!("Added {} documents to index '{}'", documents.len(), index_name);
        Ok(())
    }

    /// Delete documents by filter
    pub async fn delete_by_filter(&self, index_name: &str, filter: &str) -> Result<()> {
        let index = self.client.index(index_name);

        // First search for documents matching the filter
        let results: SearchResults<SearchDocument> = index
            .search()
            .with_filter(filter)
            .with_limit(1000)
            .execute()
            .await?;

        if results.hits.is_empty() {
            return Ok(());
        }

        // Collect IDs and delete
        let ids: Vec<&str> = results.hits.iter()
            .map(|h| h.result.id.as_str())
            .collect();

        let task = index.delete_documents(&ids).await?;
        task.wait_for_completion(
            &self.client,
            Some(std::time::Duration::from_millis(100)),
            Some(std::time::Duration::from_secs(TASK_TIMEOUT_LONG_SECS)),
        ).await?;

        log::info!("Deleted {} documents from index '{}' matching filter", ids.len(), index_name);
        Ok(())
    }

    /// Delete a document by ID
    pub async fn delete_document(&self, index_name: &str, doc_id: &str) -> Result<()> {
        let index = self.client.index(index_name);
        let task = index.delete_document(doc_id).await?;
        task.wait_for_completion(
            &self.client,
            Some(std::time::Duration::from_millis(100)),
            Some(std::time::Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)),
        ).await?;
        Ok(())
    }

    /// Get document count for an index
    pub async fn document_count(&self, index_name: &str) -> Result<u64> {
        let index = self.client.index(index_name);
        let stats = index.get_stats().await?;
        Ok(stats.number_of_documents as u64)
    }

    // ========================================================================
    // Search Operations
    // ========================================================================

    /// Search a single index
    pub async fn search(
        &self,
        index_name: &str,
        query: &str,
        limit: usize,
        filter: Option<&str>,
    ) -> Result<Vec<SearchResult>> {
        let index = self.client.index(index_name);

        let mut search = index.search();
        search.with_query(query).with_limit(limit);

        if let Some(f) = filter {
            search.with_filter(f);
        }

        let results: SearchResults<SearchDocument> = search.execute().await?;

        let search_results: Vec<SearchResult> = results.hits
            .into_iter()
            .enumerate()
            .map(|(i, hit)| SearchResult {
                document: hit.result,
                // Meilisearch doesn't give explicit scores in basic search
                // Use position-based scoring
                score: 1.0 - (i as f32 * 0.1).min(0.9),
                index: index_name.to_string(),
            })
            .collect();

        Ok(search_results)
    }

    /// Hybrid search (keyword + semantic) on a single index
    pub async fn hybrid_search(
        &self,
        index_name: &str,
        query: &str,
        limit: usize,
        semantic_ratio: f32,
        embedder: Option<&str>,
    ) -> Result<Vec<SearchResult>> {
        // Build hybrid search request manually via HTTP
        // as the SDK may not fully support hybrid search syntax
        let url = format!("{}/indexes/{}/search", self.host, index_name);
        let client = reqwest::Client::new();

        let body = serde_json::json!({
            "q": query,
            "limit": limit,
            "hybrid": {
                "semanticRatio": semantic_ratio,
                "embedder": embedder.unwrap_or("default")
            }
        });

        let mut request = client.post(&url).json(&body);

        if let Some(key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response = request.send().await
            .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;

        if !response.status().is_success() {
            // Fall back to regular search if hybrid not supported
            return self.search(index_name, query, limit, None).await;
        }

        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct HybridResponse {
            hits: Vec<SearchDocument>,
            #[serde(rename = "processingTimeMs")]
            processing_time_ms: Option<u64>,
        }

        let result: HybridResponse = response.json().await
            .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;

        let search_results: Vec<SearchResult> = result.hits
            .into_iter()
            .enumerate()
            .map(|(i, doc)| SearchResult {
                document: doc,
                score: 1.0 - (i as f32 * 0.1).min(0.9),
                index: index_name.to_string(),
            })
            .collect();

        Ok(search_results)
    }

    /// Federated search across multiple indexes
    pub async fn federated_search(
        &self,
        query: &str,
        indexes: &[&str],
        limit_per_index: usize,
    ) -> Result<FederatedResults> {
        let start = std::time::Instant::now();
        let mut all_results = Vec::new();

        // Search each index and merge results
        for index_name in indexes {
            match self.search(index_name, query, limit_per_index, None).await {
                Ok(results) => all_results.extend(results),
                Err(e) => {
                    log::warn!("Search in index '{}' failed: {}", index_name, e);
                }
            }
        }

        // Sort by score descending
        all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        let total_hits = all_results.len();
        let processing_time = start.elapsed().as_millis() as u64;

        Ok(FederatedResults {
            results: all_results,
            total_hits,
            processing_time_ms: processing_time,
        })
    }

    /// Search all content indexes (rules, fiction, documents)
    pub async fn search_all(&self, query: &str, limit: usize) -> Result<FederatedResults> {
        self.federated_search(
            query,
            &[INDEX_RULES, INDEX_FICTION, INDEX_DOCUMENTS],
            limit / 3 + 1,
        ).await
    }

    // ========================================================================
    // Index Selection Helpers
    // ========================================================================

    /// Select appropriate index based on source type
    pub fn select_index_for_source_type(source_type: &str) -> &'static str {
        match source_type.to_lowercase().as_str() {
            "rule" | "rules" | "rulebook" | "mechanics" => INDEX_RULES,
            "fiction" | "lore" | "story" | "narrative" => INDEX_FICTION,
            "chat" | "conversation" | "message" => INDEX_CHAT,
            _ => INDEX_DOCUMENTS,
        }
    }

    /// Get all index names
    pub fn all_indexes() -> Vec<&'static str> {
        vec![INDEX_RULES, INDEX_FICTION, INDEX_CHAT, INDEX_DOCUMENTS]
    }

    // ========================================================================
    // Statistics
    // ========================================================================

    /// Get statistics for all indexes
    pub async fn get_all_stats(&self) -> Result<HashMap<String, u64>> {
        let mut stats = HashMap::new();

        for index_name in Self::all_indexes() {
            match self.document_count(index_name).await {
                Ok(count) => { stats.insert(index_name.to_string(), count); }
                Err(_) => { stats.insert(index_name.to_string(), 0); }
            }
        }

        Ok(stats)
    }

    /// Clear all documents from an index
    pub async fn clear_index(&self, index_name: &str) -> Result<()> {
        let index = self.client.index(index_name);
        let task = index.delete_all_documents().await?;

        // Wait with explicit timeout (10 minutes) and polling interval (100ms)
        task.wait_for_completion(
            &self.client,
            Some(std::time::Duration::from_millis(100)),
            Some(std::time::Duration::from_secs(TASK_TIMEOUT_LONG_SECS)),
        ).await?;

        log::info!("Cleared all documents from index '{}'", index_name);
        Ok(())
    }

    // ========================================================================
    // Library Document Operations (Meilisearch-based persistence)
    // ========================================================================

    /// Save library document metadata to Meilisearch
    pub async fn save_library_document(&self, doc: &LibraryDocumentMetadata) -> Result<()> {
        let index = self.client.index(INDEX_LIBRARY_METADATA);
        let task = index.add_documents(&[doc], Some("id")).await?;
        task.wait_for_completion(
            &self.client,
            Some(std::time::Duration::from_millis(100)),
            Some(std::time::Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)),
        ).await?;
        log::info!("Saved library document metadata: {} ({})", doc.name, doc.id);
        Ok(())
    }

    /// List all library documents from Meilisearch
    pub async fn list_library_documents(&self) -> Result<Vec<LibraryDocumentMetadata>> {
        let index = self.client.index(INDEX_LIBRARY_METADATA);

        // Use search with empty query to get all documents, sorted by ingested_at
        let results: SearchResults<LibraryDocumentMetadata> = index
            .search()
            .with_query("")
            .with_limit(1000)
            .with_sort(&["ingested_at:desc"])
            .execute()
            .await?;

        let docs: Vec<LibraryDocumentMetadata> = results.hits
            .into_iter()
            .map(|hit| hit.result)
            .collect();

        Ok(docs)
    }

    /// Get a single library document by ID
    pub async fn get_library_document(&self, doc_id: &str) -> Result<Option<LibraryDocumentMetadata>> {
        let index = self.client.index(INDEX_LIBRARY_METADATA);

        match index.get_document::<LibraryDocumentMetadata>(doc_id).await {
            Ok(doc) => Ok(Some(doc)),
            Err(meilisearch_sdk::errors::Error::Meilisearch(e)) if e.error_code == meilisearch_sdk::errors::ErrorCode::DocumentNotFound => {
                Ok(None)
            }
            Err(e) => Err(SearchError::from(e)),
        }
    }

    /// Delete a library document from Meilisearch (metadata only)
    pub async fn delete_library_document(&self, doc_id: &str) -> Result<()> {
        let index = self.client.index(INDEX_LIBRARY_METADATA);
        let task = index.delete_document(doc_id).await?;
        task.wait_for_completion(
            &self.client,
            Some(std::time::Duration::from_millis(100)),
            Some(std::time::Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)),
        ).await?;
        log::info!("Deleted library document metadata: {}", doc_id);
        Ok(())
    }

    /// Delete library document and all its content chunks from the content index
    pub async fn delete_library_document_with_content(&self, doc_id: &str) -> Result<()> {
        // First get the document to find which content index it used
        if let Some(doc) = self.get_library_document(doc_id).await? {
            // Delete content chunks by source filter
            self.delete_by_filter(&doc.content_index, &format!("source = \"{}\"", doc.name)).await?;
            log::info!("Deleted {} content chunks from index '{}'", doc.name, doc.content_index);
        }

        // Delete the metadata
        self.delete_library_document(doc_id).await?;

        Ok(())
    }

    /// Get library document count
    pub async fn library_document_count(&self) -> Result<u64> {
        self.document_count(INDEX_LIBRARY_METADATA).await
    }

    /// Rebuild library metadata from existing content indices.
    ///
    /// Scans all content indices for unique sources and creates metadata entries
    /// for any sources that don't already have entries in library_metadata.
    /// Derives page_count from max(page_number) in chunks.
    pub async fn rebuild_library_metadata(&self) -> Result<Vec<LibraryDocumentMetadata>> {
        // Use all_indexes() to ensure we scan all content indices
        let content_indices = Self::all_indexes();
        // Track: (index_name, chunk_count, char_count, max_page_number)
        let mut discovered_sources: std::collections::HashMap<String, (String, u32, u64, u32)> = std::collections::HashMap::new();

        // Get existing library documents to avoid duplicates
        let existing = self.list_library_documents().await.unwrap_or_default();
        let existing_names: std::collections::HashSet<String> = existing.iter().map(|d| d.name.clone()).collect();

        log::info!("Rebuilding library metadata, {} existing entries", existing.len());

        for index_name in content_indices {
            // Check if index exists
            if self.document_count(index_name).await.unwrap_or(0) == 0 {
                continue;
            }

            log::info!("Scanning index '{}' for sources...", index_name);

            // Get all unique sources by querying with facets
            // Meilisearch doesn't have direct facet aggregation, so we'll paginate through docs
            let index = self.client.index(index_name);
            let mut offset = 0;
            let limit = 1000;

            loop {
                let results: SearchResults<SearchDocument> = index
                    .search()
                    .with_query("")
                    .with_limit(limit)
                    .with_offset(offset)
                    .execute()
                    .await?;

                if results.hits.is_empty() {
                    break;
                }

                for hit in results.hits {
                    let doc = hit.result;
                    let entry = discovered_sources.entry(doc.source.clone())
                        .or_insert((index_name.to_string(), 0, 0, 0));
                    entry.1 += 1; // chunk count
                    entry.2 += doc.content.len() as u64; // character count
                    // Track max page number to estimate total pages
                    if let Some(page_num) = doc.page_number {
                        entry.3 = entry.3.max(page_num);
                    }
                }

                offset += limit;

                // Safety limit
                if offset > 50000 {
                    log::warn!("Reached safety limit scanning index '{}'", index_name);
                    break;
                }
            }
        }

        log::info!("Found {} unique sources across all indices", discovered_sources.len());

        // Create metadata for new sources
        let mut created = Vec::new();
        for (source_name, (index_name, chunk_count, char_count, max_page)) in discovered_sources {
            if existing_names.contains(&source_name) {
                log::debug!("Skipping existing source: {}", source_name);
                continue;
            }

            // Infer source type from index or filename
            let source_type = if index_name == INDEX_RULES {
                "rulebook"
            } else if index_name == INDEX_FICTION {
                "fiction"
            } else if index_name == INDEX_CHAT {
                "chat"
            } else if source_name.to_lowercase().ends_with(".pdf") {
                "rulebook" // PDFs are typically rulebooks
            } else if source_name.to_lowercase().ends_with(".epub") {
                "fiction"  // EPUBs are typically fiction
            } else {
                "documents"
            }.to_string();

            // Derive page count from max page number seen in chunks
            // If no page numbers found, estimate from chunk count (assuming ~4 chunks/page)
            let page_count = if max_page > 0 {
                max_page
            } else {
                (chunk_count / 4).max(1)
            };

            let metadata = LibraryDocumentMetadata {
                id: uuid::Uuid::new_v4().to_string(),
                name: source_name.clone(),
                source_type,
                file_path: None, // Unknown for legacy docs
                page_count,
                chunk_count,
                character_count: char_count,
                content_index: index_name,
                status: "ready".to_string(),
                error_message: None,
                ingested_at: chrono::Utc::now().to_rfc3339(),
            };

            if let Err(e) = self.save_library_document(&metadata).await {
                log::warn!("Failed to save metadata for '{}': {}", source_name, e);
            } else {
                log::info!("Created metadata for legacy source: {}", source_name);
                created.push(metadata);
            }
        }

        log::info!("Created {} new library metadata entries", created.len());
        Ok(created)
    }

    // ========================================================================
    // TTRPG-Specific Operations
    // ========================================================================

    /// Configure an index for TTRPG document search with appropriate filterable fields
    pub async fn configure_ttrpg_index(&self, index_name: &str) -> Result<()> {
        self.ensure_index(index_name, Some("id")).await?;

        let settings = Settings::new()
            .with_searchable_attributes(["content", "source", "element_type", "metadata"])
            .with_filterable_attributes([
                // TTRPG-specific filters
                "damage_types",
                "creature_types",
                "conditions",
                "alignments",
                "rarities",
                "sizes",
                "spell_schools",
                "element_type",
                "challenge_rating",
                "level",
                "game_system",
                // Standard filters
                "source",
                "source_type",
                "page_number",
                "campaign_id",
                "session_id",
                "created_at",
            ])
            .with_sortable_attributes([
                "challenge_rating",
                "level",
                "created_at",
            ]);

        let index = self.client.index(index_name);
        let task = index.set_settings(&settings).await?;
        task.wait_for_completion(
            &self.client,
            Some(std::time::Duration::from_millis(100)),
            Some(std::time::Duration::from_secs(30)),
        ).await?;

        log::info!("Configured TTRPG index '{}'", index_name);
        Ok(())
    }

    /// Configure a raw document index for the two-phase ingestion pipeline.
    /// Raw indexes store page-level documents and need sorting by page_number.
    pub async fn ensure_raw_index(&self, index_name: &str) -> Result<Index> {
        // Create index if it doesn't exist
        let index = self.ensure_index(index_name, Some("id")).await?;

        // Configure settings for raw document indexes
        let settings = Settings::new()
            .with_searchable_attributes(["raw_content", "source_slug"])
            .with_sortable_attributes(["page_number"]);

        let task = index.set_settings(&settings).await?;
        task.wait_for_completion(
            &self.client,
            Some(std::time::Duration::from_millis(100)),
            Some(std::time::Duration::from_secs(30)),
        ).await?;

        log::info!("Configured raw index '{}'", index_name);
        Ok(index)
    }

    /// Configure a chunks index for the two-phase ingestion pipeline.
    /// Chunks indexes store semantic chunks with TTRPG metadata.
    pub async fn ensure_chunks_index(&self, index_name: &str) -> Result<Index> {
        // Create index if it doesn't exist
        let index = self.ensure_index(index_name, Some("id")).await?;

        // Configure settings for chunked document indexes
        let settings = Settings::new()
            .with_searchable_attributes(["content", "source_slug", "book_title", "game_system"])
            .with_sortable_attributes(["page_start", "chunk_index"]);

        let task = index.set_settings(&settings).await?;
        task.wait_for_completion(
            &self.client,
            Some(std::time::Duration::from_millis(100)),
            Some(std::time::Duration::from_secs(30)),
        ).await?;

        log::info!("Configured chunks index '{}'", index_name);
        Ok(index)
    }

    /// Add TTRPG documents with game-specific metadata
    pub async fn add_ttrpg_documents(
        &self,
        index_name: &str,
        documents: Vec<TTRPGSearchDocument>,
    ) -> Result<()> {
        if documents.is_empty() {
            return Ok(());
        }

        let index = self.client.index(index_name);
        let task = index.add_documents(&documents, Some("id")).await?;

        task.wait_for_completion(
            &self.client,
            Some(std::time::Duration::from_millis(100)),
            Some(std::time::Duration::from_secs(30)),
        ).await?;

        log::info!("Added {} TTRPG documents to index '{}'", documents.len(), index_name);
        Ok(())
    }

    /// Search TTRPG documents with game-specific filters
    pub async fn search_ttrpg(
        &self,
        index_name: &str,
        query: &str,
        limit: usize,
        filter: Option<&str>,
    ) -> Result<Vec<TTRPGSearchResult>> {
        let index = self.client.index(index_name);

        let mut search = index.search();
        search.with_query(query).with_limit(limit);

        if let Some(f) = filter {
            search.with_filter(f);
        }

        let results: SearchResults<TTRPGSearchDocument> = search.execute().await?;

        let search_results: Vec<TTRPGSearchResult> = results.hits
            .into_iter()
            .enumerate()
            .map(|(i, hit)| TTRPGSearchResult {
                document: hit.result,
                score: 1.0 - (i as f32 * 0.1).min(0.9),
                index: index_name.to_string(),
            })
            .collect();

        Ok(search_results)
    }
}

// ============================================================================
// TTRPG Document Types
// ============================================================================

/// TTRPG-specific searchable document with game metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TTRPGSearchDocument {
    /// Base document fields
    #[serde(flatten)]
    pub base: SearchDocument,

    // TTRPG-specific filterable fields
    /// Damage types mentioned (fire, cold, radiant, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub damage_types: Vec<String>,

    /// Creature types (humanoid, undead, dragon, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub creature_types: Vec<String>,

    /// Conditions (poisoned, frightened, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<String>,

    /// Alignments (lawful good, chaotic evil, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alignments: Vec<String>,

    /// Item rarities (common, rare, legendary, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rarities: Vec<String>,

    /// Size categories (tiny, small, medium, large, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sizes: Vec<String>,

    /// Spell schools (evocation, necromancy, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spell_schools: Vec<String>,

    /// Challenge rating for monsters/encounters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub challenge_rating: Option<f32>,

    /// Level (spell level, class level, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<u32>,

    /// Element type (stat_block, random_table, spell, etc.)
    #[serde(default)]
    pub element_type: String,

    /// Detected game system (dnd5e, pf2e, etc.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_system: Option<String>,

    /// Section hierarchy path
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section_path: Option<String>,
}

impl TTRPGSearchDocument {
    /// Create a new TTRPG document from base document and attributes
    pub fn new(base: SearchDocument, element_type: &str) -> Self {
        Self {
            base,
            damage_types: Vec::new(),
            creature_types: Vec::new(),
            conditions: Vec::new(),
            alignments: Vec::new(),
            rarities: Vec::new(),
            sizes: Vec::new(),
            spell_schools: Vec::new(),
            challenge_rating: None,
            level: None,
            element_type: element_type.to_string(),
            game_system: None,
            section_path: None,
        }
    }

    /// Create from a content chunk with TTRPG attributes
    pub fn from_chunk(
        chunk: &crate::ingestion::ContentChunk,
        attributes: &crate::ingestion::TTRPGAttributes,
        element_type: &str,
        game_system: Option<&str>,
    ) -> Self {
        let filterable = attributes.to_filterable_fields();

        let base = SearchDocument {
            id: chunk.id.clone(),
            content: chunk.content.clone(),
            source: chunk.source_id.clone(),
            source_type: element_type.to_string(),
            page_number: chunk.page_number,
            chunk_index: Some(chunk.chunk_index as u32),
            campaign_id: None,
            session_id: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            metadata: chunk.metadata.clone(),
            // TTRPG metadata populated from game_system parameter
            game_system: game_system.map(|s| {
                // Try to get display name from game system
                crate::ingestion::ttrpg::game_detector::GameSystem::from_str(s)
                    .map(|gs| gs.display_name().to_string())
                    .unwrap_or_else(|| s.to_string())
            }),
            game_system_id: game_system.map(|s| s.to_string()),
            ..Default::default()
        };

        Self {
            base,
            damage_types: filterable.damage_types,
            creature_types: filterable.creature_types,
            conditions: filterable.conditions,
            alignments: filterable.alignments,
            rarities: filterable.rarities,
            sizes: filterable.sizes,
            spell_schools: filterable.spell_schools,
            challenge_rating: filterable.challenge_rating,
            level: None,
            element_type: element_type.to_string(),
            game_system: game_system.map(|s| s.to_string()),
            section_path: chunk.metadata.get("section_path").cloned(),
        }
    }
}

/// TTRPG search result with full document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TTRPGSearchResult {
    pub document: TTRPGSearchDocument,
    pub score: f32,
    pub index: String,
}

/// Index name for TTRPG content
pub const INDEX_TTRPG: &str = "ttrpg";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_selection() {
        assert_eq!(SearchClient::select_index_for_source_type("rules"), INDEX_RULES);
        assert_eq!(SearchClient::select_index_for_source_type("fiction"), INDEX_FICTION);
        assert_eq!(SearchClient::select_index_for_source_type("chat"), INDEX_CHAT);
        assert_eq!(SearchClient::select_index_for_source_type("pdf"), INDEX_DOCUMENTS);
    }

    #[test]
    fn test_search_document_serialization() {
        let doc = SearchDocument {
            id: "test-1".to_string(),
            content: "Test content".to_string(),
            source: "test.pdf".to_string(),
            source_type: "document".to_string(),
            page_number: Some(1),
            chunk_index: Some(0),
            campaign_id: None,
            session_id: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            metadata: HashMap::new(),
            ..Default::default()
        };

        let json = serde_json::to_string(&doc).unwrap();
        assert!(json.contains("test-1"));
        assert!(json.contains("Test content"));
    }

    #[test]
    fn test_ttrpg_document_serialization() {
        let base = SearchDocument {
            id: "ttrpg-1".to_string(),
            content: "Goblin stat block".to_string(),
            source: "monster_manual.pdf".to_string(),
            source_type: "stat_block".to_string(),
            page_number: Some(42),
            chunk_index: Some(0),
            campaign_id: None,
            session_id: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            metadata: HashMap::new(),
            ..Default::default()
        };

        let mut doc = TTRPGSearchDocument::new(base, "stat_block");
        doc.damage_types = vec!["slashing".to_string()];
        doc.creature_types = vec!["humanoid".to_string()];
        doc.sizes = vec!["small".to_string()];
        doc.challenge_rating = Some(0.25);
        doc.game_system = Some("dnd5e".to_string());

        let json = serde_json::to_string(&doc).unwrap();
        assert!(json.contains("ttrpg-1"));
        assert!(json.contains("stat_block"));
        assert!(json.contains("slashing"));
        assert!(json.contains("humanoid"));
        assert!(json.contains("dnd5e"));
    }

    #[test]
    fn test_ttrpg_document_round_trip() {
        let base = SearchDocument {
            id: "round-trip-1".to_string(),
            content: "Fire bolt cantrip".to_string(),
            source: "phb.pdf".to_string(),
            source_type: "spell".to_string(),
            page_number: Some(100),
            chunk_index: Some(5),
            campaign_id: None,
            session_id: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            metadata: HashMap::new(),
            ..Default::default()
        };

        let mut doc = TTRPGSearchDocument::new(base, "spell");
        doc.damage_types = vec!["fire".to_string()];
        doc.spell_schools = vec!["evocation".to_string()];
        doc.level = Some(0);

        let json = serde_json::to_string(&doc).unwrap();
        let parsed: TTRPGSearchDocument = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.base.id, "round-trip-1");
        assert_eq!(parsed.element_type, "spell");
        assert_eq!(parsed.damage_types, vec!["fire"]);
        assert_eq!(parsed.level, Some(0));
    }

    #[test]
    fn test_ttrpg_index_constant() {
        assert_eq!(INDEX_TTRPG, "ttrpg");
    }
}
