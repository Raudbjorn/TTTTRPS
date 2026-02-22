//! Search Client
//!
//! Core Meilisearch client implementation for document indexing and search operations.

use meilisearch_sdk::client::Client;
use meilisearch_sdk::indexes::Index;
use meilisearch_sdk::search::SearchResults;
use meilisearch_sdk::settings::Settings;
use serde::Deserialize;
use std::collections::HashMap;

use super::config::{
    all_indexes, build_embedder_json, ollama_embedding_dimensions, EmbedderConfig, INDEX_CHAT,
    INDEX_DOCUMENTS, INDEX_FICTION, INDEX_LIBRARY_METADATA, INDEX_RULES, TASK_TIMEOUT_LONG_SECS,
    TASK_TIMEOUT_SHORT_SECS,
};
use super::error::{Result, SearchError};
use super::library::LibraryRepositoryImpl;
use super::models::{FederatedResults, LibraryDocumentMetadata, SearchDocument, SearchResult};
use super::ttrpg::{
    TTRPGSearchDocument, TTRPGSearchResult, TTRPG_FILTERABLE_ATTRIBUTES,
    TTRPG_SEARCHABLE_ATTRIBUTES, TTRPG_SORTABLE_ATTRIBUTES,
};

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

    // ========================================================================
    // Health & Connection
    // ========================================================================

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

    // ========================================================================
    // Index Management
    // ========================================================================

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
                let task = self.client.create_index(name, primary_key).await?;
                task.wait_for_completion(
                    &self.client,
                    Some(std::time::Duration::from_millis(100)),
                    Some(std::time::Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)),
                )
                .await?;
                Ok(self.client.index(name))
            }
        }
    }

    /// Delete an index entirely
    pub async fn delete_index(&self, name: &str) -> Result<()> {
        match self.client.delete_index(name).await {
            Ok(task) => {
                task.wait_for_completion(
                    &self.client,
                    Some(std::time::Duration::from_millis(100)),
                    Some(std::time::Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)),
                )
                .await?;
                log::info!("Deleted index '{}'", name);
                Ok(())
            }
            Err(meilisearch_sdk::errors::Error::Meilisearch(err))
                if err.error_code == meilisearch_sdk::errors::ErrorCode::IndexNotFound =>
            {
                // Index doesn't exist - that's fine for deletion
                log::debug!("Index '{}' already doesn't exist", name);
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Initialize all specialized indexes with appropriate settings
    pub async fn initialize_indexes(&self) -> Result<()> {
        // Enable experimental features (vectorStore) required for hybrid search
        let url = format!("{}/experimental-features", self.host);
        let client = reqwest::Client::new();
        let mut request = client.patch(&url).json(&serde_json::json!({
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
            .with_filterable_attributes([
                "source",
                "source_type",
                "campaign_id",
                "session_id",
                "created_at",
            ])
            .with_sortable_attributes(["created_at"]);

        // Apply settings to content indexes
        for index_name in [INDEX_RULES, INDEX_FICTION, INDEX_CHAT, INDEX_DOCUMENTS] {
            let index = self.client.index(index_name);
            let task = index.set_settings(&base_settings).await?;
            task.wait_for_completion(
                &self.client,
                Some(std::time::Duration::from_millis(100)),
                Some(std::time::Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)),
            )
            .await?;
        }

        // Configure library metadata index settings
        let library_settings = Settings::new()
            .with_searchable_attributes([
                "name",
                "source_type",
                "file_path",
                // TTRPG metadata (searchable for discovery)
                "game_system",
                "setting",
                "content_type",
                "publisher",
            ])
            .with_filterable_attributes([
                "source_type",
                "status",
                "content_index",
                "ingested_at",
                // TTRPG metadata (filterable for organization)
                "game_system",
                "setting",
                "content_type",
                "publisher",
            ])
            .with_sortable_attributes(["ingested_at", "name", "page_count", "chunk_count"]);

        let library_index = self.client.index(INDEX_LIBRARY_METADATA);
        let task = library_index.set_settings(&library_settings).await?;
        task.wait_for_completion(
            &self.client,
            Some(std::time::Duration::from_millis(100)),
            Some(std::time::Duration::from_secs(30)),
        )
        .await?;

        log::info!(
            "Initialized Meilisearch indexes: rules, fiction, chat, documents, library_metadata"
        );
        Ok(())
    }

    // ========================================================================
    // Embedder Configuration
    // ========================================================================

    /// Configure an embedder for semantic search on an index
    pub async fn configure_embedder(
        &self,
        index_name: &str,
        embedder_name: &str,
        config: &EmbedderConfig,
    ) -> Result<()> {
        let embedder_json = build_embedder_json(config);

        // Use PATCH to update embedders setting
        let url = format!("{}/indexes/{}/settings/embedders", self.host, index_name);
        let client = reqwest::Client::new();

        let mut request = client
            .patch(&url)
            .json(&serde_json::json!({ embedder_name: embedder_json }));

        if let Some(key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response = request
            .send()
            .await
            .map_err(|e| SearchError::ConfigError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SearchError::ConfigError(format!(
                "Failed to configure embedder: {}",
                error_text
            )));
        }

        log::info!(
            "Configured embedder '{}' for index '{}'",
            embedder_name,
            index_name
        );
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
        let content_indexes = all_indexes();

        for index_name in content_indexes {
            match self.configure_embedder(index_name, "ollama", &config).await {
                Ok(_) => {
                    log::info!(
                        "Configured Ollama embedder on index '{}' with model '{}'",
                        index_name,
                        model
                    );
                    configured.push(index_name.to_string());
                }
                Err(e) => {
                    log::warn!("Failed to configure embedder on '{}': {}", index_name, e);
                }
            }
        }

        if configured.is_empty() {
            return Err(SearchError::ConfigError(
                "Failed to configure any indexes".to_string(),
            ));
        }

        Ok(configured)
    }

    /// Configure Copilot embeddings on all content indexes
    ///
    /// This configures Meilisearch to use GitHub Copilot for AI-powered semantic search.
    /// The embedder is configured as a REST source calling the Copilot API directly.
    pub async fn setup_copilot_embeddings(
        &self,
        model: &str,
        dimensions: u32,
        api_key: &str,
    ) -> Result<Vec<String>> {
        let config = EmbedderConfig::Copilot {
            api_key: api_key.to_string(),
            model: model.to_string(),
            dimensions,
        };

        let mut configured = Vec::new();
        let content_indexes = all_indexes();

        for index_name in content_indexes {
            match self.configure_embedder(index_name, "copilot", &config).await {
                Ok(_) => {
                    log::info!(
                        "Configured Copilot embedder on index '{}' with model '{}'",
                        index_name,
                        model
                    );
                    configured.push(index_name.to_string());
                }
                Err(e) => {
                    log::warn!("Failed to configure embedder on '{}': {}", index_name, e);
                }
            }
        }

        if configured.is_empty() {
            return Err(SearchError::ConfigError(
                "Failed to configure any indexes".to_string(),
            ));
        }

        Ok(configured)
    }

    /// Get current embedder configuration for an index
    pub async fn get_embedder_settings(
        &self,
        index_name: &str,
    ) -> Result<Option<serde_json::Value>> {
        let url = format!("{}/indexes/{}/settings/embedders", self.host, index_name);
        let client = reqwest::Client::new();

        let mut request = client.get(&url);
        if let Some(key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response = request
            .send()
            .await
            .map_err(|e| SearchError::ConfigError(e.to_string()))?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let settings: serde_json::Value = response
            .json()
            .await
            .map_err(|e| SearchError::ConfigError(e.to_string()))?;

        Ok(Some(settings))
    }

    // ========================================================================
    // Document Operations
    // ========================================================================

    /// Add documents to an index
    pub async fn add_documents(
        &self,
        index_name: &str,
        documents: Vec<SearchDocument>,
    ) -> Result<()> {
        if documents.is_empty() {
            log::warn!(
                "add_documents called with empty document list for index '{}' - nothing indexed",
                index_name
            );
            return Ok(());
        }

        let index = self.client.index(index_name);
        let task = index.add_documents(&documents, Some("id")).await?;

        // Wait with explicit timeout (10 minutes) and polling interval (100ms)
        task.wait_for_completion(
            &self.client,
            Some(std::time::Duration::from_millis(100)),
            Some(std::time::Duration::from_secs(TASK_TIMEOUT_LONG_SECS)),
        )
        .await?;

        log::info!(
            "Added {} documents to index '{}'",
            documents.len(),
            index_name
        );
        Ok(())
    }

    /// Delete documents by filter
    ///
    /// Paginates through all matching documents to handle cases where more than
    /// 1000 documents match the filter.
    pub async fn delete_by_filter(&self, index_name: &str, filter: &str) -> Result<()> {
        let index = self.client.index(index_name);
        let mut total_deleted = 0;
        const PAGE_SIZE: usize = 1000;

        loop {
            // Search for documents matching the filter
            let results: SearchResults<SearchDocument> = index
                .search()
                .with_filter(filter)
                .with_limit(PAGE_SIZE)
                .execute()
                .await?;

            if results.hits.is_empty() {
                break;
            }

            // Collect IDs and delete
            let ids: Vec<&str> = results.hits.iter().map(|h| h.result.id.as_str()).collect();
            let batch_count = ids.len();

            let task = index.delete_documents(&ids).await?;
            task.wait_for_completion(
                &self.client,
                Some(std::time::Duration::from_millis(100)),
                Some(std::time::Duration::from_secs(TASK_TIMEOUT_LONG_SECS)),
            )
            .await?;

            total_deleted += batch_count;

            // If we got fewer than PAGE_SIZE results, we've processed all matching docs
            if batch_count < PAGE_SIZE {
                break;
            }
        }

        if total_deleted > 0 {
            log::info!(
                "Deleted {} documents from index '{}' matching filter",
                total_deleted,
                index_name
            );
        }
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
        )
        .await?;
        Ok(())
    }

    /// Get document count for an index
    pub async fn document_count(&self, index_name: &str) -> Result<u64> {
        let index = self.client.index(index_name);
        let stats = index.get_stats().await?;
        Ok(stats.number_of_documents as u64)
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
        )
        .await?;

        log::info!("Cleared all documents from index '{}'", index_name);
        Ok(())
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

        let search_results: Vec<SearchResult> = results
            .hits
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

        let response = request
            .send()
            .await
            .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;

        if !response.status().is_success() {
            // Fall back to regular search if hybrid not supported
            return self.search(index_name, query, limit, None).await;
        }

        #[derive(Deserialize)]
        struct HybridResponse {
            hits: Vec<SearchDocument>,
            #[serde(rename = "processingTimeMs")]
            _processing_time_ms: Option<u64>,
        }

        let result: HybridResponse = response
            .json()
            .await
            .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;

        let search_results: Vec<SearchResult> = result
            .hits
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
        all_results
            .sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

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
        let indices = [INDEX_RULES, INDEX_FICTION, INDEX_DOCUMENTS];
        // Ceiling division to distribute results evenly across indices
        let per_index_limit = (limit + indices.len() - 1) / indices.len();
        self.federated_search(query, &indices, per_index_limit).await
    }

    // ========================================================================
    // Index Selection Helpers (static methods)
    // ========================================================================

    /// Select appropriate index based on source type
    pub fn select_index_for_source_type(source_type: &str) -> &'static str {
        super::config::select_index_for_source_type(source_type)
    }

    /// Get all index names
    pub fn all_indexes() -> Vec<&'static str> {
        all_indexes()
    }

    // ========================================================================
    // Statistics
    // ========================================================================

    /// Get statistics for all indexes (including dynamic per-document indexes)
    pub async fn get_all_stats(&self) -> Result<HashMap<String, u64>> {
        let mut stats = HashMap::new();

        // Get all indexes from Meilisearch with pagination to ensure we don't miss any
        let all_meili_indexes: Vec<String> = match self.list_all_index_names().await {
            Ok(indexes) => indexes,
            Err(e) => {
                log::warn!("Failed to list all indexes: {}, falling back to static list", e);
                Self::all_indexes().iter().map(|s| s.to_string()).collect()
            }
        };

        for index_name in all_meili_indexes {
            // Skip internal indexes that shouldn't be shown in stats
            if index_name.ends_with("-raw") || index_name == "library_metadata" {
                continue;
            }
            match self.document_count(&index_name).await {
                Ok(count) => {
                    stats.insert(index_name, count);
                }
                Err(_) => {
                    stats.insert(index_name, 0);
                }
            }
        }

        Ok(stats)
    }

    /// List all index names with proper pagination handling
    async fn list_all_index_names(&self) -> Result<Vec<String>> {
        let client = reqwest::Client::new();
        let mut all_names = Vec::new();
        let mut offset = 0u32;
        const PAGE_SIZE: u32 = 100;

        loop {
            let url = format!("{}/indexes?limit={}&offset={}", self.host, PAGE_SIZE, offset);
            let mut request = client.get(&url);

            if let Some(key) = &self.api_key {
                request = request.header("Authorization", format!("Bearer {}", key));
            }

            let response = request
                .send()
                .await
                .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;

            if !response.status().is_success() {
                return Err(SearchError::MeilisearchError(format!(
                    "Failed to list indexes: HTTP {}",
                    response.status()
                )));
            }

            #[derive(Deserialize)]
            struct IndexInfo {
                uid: String,
            }

            #[derive(Deserialize)]
            struct IndexesResponse {
                results: Vec<IndexInfo>,
            }

            let data: IndexesResponse = response
                .json()
                .await
                .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;

            let count = data.results.len();
            all_names.extend(data.results.into_iter().map(|idx| idx.uid));

            // If we got fewer than PAGE_SIZE, we've reached the end
            if (count as u32) < PAGE_SIZE {
                break;
            }

            offset += PAGE_SIZE;

            // Safety limit to prevent infinite loops (unlikely but defensive)
            if offset > 10000 {
                log::warn!("Hit safety limit while paginating indexes");
                break;
            }
        }

        Ok(all_names)
    }

    // ========================================================================
    // Library Document Operations (delegated to LibraryRepositoryImpl)
    // ========================================================================

    fn library_repo(&self) -> LibraryRepositoryImpl<'_> {
        LibraryRepositoryImpl::new(&self.client, &self.host)
    }

    /// Save library document metadata to Meilisearch
    pub async fn save_library_document(&self, doc: &LibraryDocumentMetadata) -> Result<()> {
        self.library_repo().save(doc).await
    }

    /// List all library documents from Meilisearch
    pub async fn list_library_documents(&self) -> Result<Vec<LibraryDocumentMetadata>> {
        self.library_repo().list().await
    }

    /// Get a single library document by ID
    pub async fn get_library_document(
        &self,
        doc_id: &str,
    ) -> Result<Option<LibraryDocumentMetadata>> {
        self.library_repo().get(doc_id).await
    }

    /// Delete a library document from Meilisearch (metadata only)
    pub async fn delete_library_document(&self, doc_id: &str) -> Result<()> {
        self.library_repo().delete(doc_id).await
    }

    /// Delete library document and all its content chunks from the content index
    pub async fn delete_library_document_with_content(&self, doc_id: &str) -> Result<()> {
        self.library_repo().delete_with_content(doc_id).await
    }

    /// Get library document count
    pub async fn library_document_count(&self) -> Result<u64> {
        self.library_repo().count().await
    }

    /// Rebuild library metadata from existing content indices
    pub async fn rebuild_library_metadata(&self) -> Result<Vec<LibraryDocumentMetadata>> {
        self.library_repo().rebuild_metadata().await
    }

    // ========================================================================
    // TTRPG-Specific Operations
    // ========================================================================

    /// Configure an index for TTRPG document search with appropriate filterable fields
    pub async fn configure_ttrpg_index(&self, index_name: &str) -> Result<()> {
        self.ensure_index(index_name, Some("id")).await?;

        let settings = Settings::new()
            .with_searchable_attributes(TTRPG_SEARCHABLE_ATTRIBUTES.to_vec())
            .with_filterable_attributes(TTRPG_FILTERABLE_ATTRIBUTES.to_vec())
            .with_sortable_attributes(TTRPG_SORTABLE_ATTRIBUTES.to_vec());

        let index = self.client.index(index_name);
        let task = index.set_settings(&settings).await?;
        task.wait_for_completion(
            &self.client,
            Some(std::time::Duration::from_millis(100)),
            Some(std::time::Duration::from_secs(30)),
        )
        .await?;

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
        )
        .await?;

        log::info!("Configured raw index '{}'", index_name);
        Ok(index)
    }

    /// Configure a chunks index for the two-phase ingestion pipeline.
    /// Chunks indexes store semantic chunks with TTRPG metadata.
    pub async fn ensure_chunks_index(&self, index_name: &str) -> Result<Index> {
        // Create index if it doesn't exist
        let index = self.ensure_index(index_name, Some("id")).await?;

        // Configure settings for chunked document indexes with v2 enhanced metadata
        let settings = Settings::new()
            .with_searchable_attributes([
                "content",
                "embedding_content", // Context-injected content for better semantic search
                "source_slug",
                "book_title",
                "game_system",
                "section_path",
                "semantic_keywords",
            ])
            .with_filterable_attributes([
                // v2 enhanced TTRPG filters
                "element_type",     // stat_block, random_table, spell, item, etc.
                "content_mode",     // crunch, fluff, mixed, example, optional, fiction
                "section_depth",    // 0=root, 1=chapter, 2=section, etc.
                "parent_sections",  // ["Chapter 1", "Monsters"]
                "cross_refs",       // ["page:47", "chapter:3"]
                "dice_expressions", // ["2d6", "1d20+5"]
                // Existing TTRPG metadata
                "game_system",
                "game_system_id",
                "content_category", // rulebook, adventure, setting, bestiary
                "mechanic_type",    // skill_check, combat, damage, etc.
                // Page/chunk tracking
                "page_start",
                "page_end",
                "source_slug",
                "source", // Filter/search by source document
            ])
            .with_sortable_attributes([
                "page_start",
                "chunk_index",
                "section_depth",
                "classification_confidence",
            ]);

        let task = index.set_settings(&settings).await?;
        task.wait_for_completion(
            &self.client,
            Some(std::time::Duration::from_millis(100)),
            Some(std::time::Duration::from_secs(30)),
        )
        .await?;

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
            Some(std::time::Duration::from_secs(TASK_TIMEOUT_LONG_SECS)),
        )
        .await?;

        log::info!(
            "Added {} TTRPG documents to index '{}'",
            documents.len(),
            index_name
        );
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

        let search_results: Vec<TTRPGSearchResult> = results
            .hits
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
