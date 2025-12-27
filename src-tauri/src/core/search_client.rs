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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchDocument {
    /// Unique document ID
    pub id: String,
    /// Text content
    pub content: String,
    /// Source file or origin
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
    /// Ollama embeddings (local)
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

    /// Check if Meilisearch is healthy
    pub async fn health_check(&self) -> bool {
        self.client.is_healthy().await
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
                task.wait_for_completion(&self.client, None, None).await?;
                Ok(self.client.index(name))
            }
        }
    }

    /// Initialize all specialized indexes with appropriate settings
    pub async fn initialize_indexes(&self) -> Result<()> {
        // Create all indexes
        for index_name in [INDEX_RULES, INDEX_FICTION, INDEX_CHAT, INDEX_DOCUMENTS] {
            self.ensure_index(index_name, Some("id")).await?;
        }

        // Configure settings for each index
        let base_settings = Settings::new()
            .with_searchable_attributes(["content", "source", "metadata"])
            .with_filterable_attributes(["source", "source_type", "campaign_id", "session_id", "created_at"])
            .with_sortable_attributes(["created_at"]);

        // Apply settings to all indexes
        for index_name in [INDEX_RULES, INDEX_FICTION, INDEX_CHAT, INDEX_DOCUMENTS] {
            let index = self.client.index(index_name);
            let task = index.set_settings(&base_settings).await?;
            task.wait_for_completion(&self.client, None, None).await?;
        }

        log::info!("Initialized Meilisearch indexes: rules, fiction, chat, documents");
        Ok(())
    }

    /// Configure an embedder for semantic search on an index
    pub async fn configure_embedder(
        &self,
        index_name: &str,
        embedder_name: &str,
        config: &EmbedderConfig,
    ) -> Result<()> {
        let index = self.client.index(index_name);

        // Build embedder settings as JSON
        let embedder_json = match config {
            EmbedderConfig::OpenAI { api_key, model, dimensions } => {
                serde_json::json!({
                    "source": "openAi",
                    "apiKey": api_key,
                    "model": model.clone().unwrap_or_else(|| "text-embedding-3-small".to_string()),
                    "dimensions": dimensions.unwrap_or(1536),
                    "documentTemplate": "{{doc.content}}"
                })
            }
            EmbedderConfig::Ollama { url, model } => {
                serde_json::json!({
                    "source": "ollama",
                    "url": url,
                    "model": model,
                    "documentTemplate": "{{doc.content}}"
                })
            }
            EmbedderConfig::HuggingFace { model } => {
                serde_json::json!({
                    "source": "huggingFace",
                    "model": model,
                    "documentTemplate": "{{doc.content}}"
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

    // ========================================================================
    // Document Operations
    // ========================================================================

    /// Add documents to an index
    pub async fn add_documents(&self, index_name: &str, documents: Vec<SearchDocument>) -> Result<()> {
        if documents.is_empty() {
            return Ok(());
        }

        let index = self.client.index(index_name);
        let task = index.add_documents(&documents, Some("id")).await?;
        task.wait_for_completion(&self.client, None, None).await?;

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
        task.wait_for_completion(&self.client, None, None).await?;

        log::info!("Deleted {} documents from index '{}' matching filter", ids.len(), index_name);
        Ok(())
    }

    /// Delete a document by ID
    pub async fn delete_document(&self, index_name: &str, doc_id: &str) -> Result<()> {
        let index = self.client.index(index_name);
        let task = index.delete_document(doc_id).await?;
        task.wait_for_completion(&self.client, None, None).await?;
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

        let mut body = serde_json::json!({
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
        task.wait_for_completion(&self.client, None, None).await?;
        log::info!("Cleared all documents from index '{}'", index_name);
        Ok(())
    }
}

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
        };

        let json = serde_json::to_string(&doc).unwrap();
        assert!(json.contains("test-1"));
        assert!(json.contains("Test content"));
    }
}
