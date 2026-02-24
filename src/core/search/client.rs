//! Search Client
//!
//! Core Meilisearch client implementation for document indexing and search operations.

use crate::core::wilysearch::engine::Engine;
use crate::core::wilysearch::traits::{Documents, Indexes, SettingsApi, Search, System};
use crate::core::wilysearch::types::{AddDocumentsQuery, DocumentQuery, SearchRequest, Settings};
use crate::core::wilysearch::error::Error as WilySearchError;
use std::sync::Arc;
use std::collections::HashMap;

use super::config::{
    all_indexes, build_embedder_json, ollama_embedding_dimensions, EmbedderConfig, INDEX_CHAT,
    INDEX_DOCUMENTS, INDEX_FICTION, INDEX_LIBRARY_METADATA, INDEX_RULES,
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

/// Central client for interacting with the embedded Meilisearch instance.
#[derive(Clone)]
pub struct SearchClient {
    /// Internal Arc-wrapped embedded engine
    meili: Arc<Engine>,
}

impl SearchClient {
    /// Initialize with an existing embedded Engine.
    pub fn new(meili: Arc<Engine>) -> Self {
        Self { meili }
    }

    /// Exposes the underlying embedded Meilisearch engine.
    pub fn get_client(&self) -> Arc<Engine> {
        self.meili.clone()
    }

    // ========================================================================
    // Health & Connection
    // ========================================================================

    /// Check if Meilisearch is healthy
    pub async fn health_check(&self) -> bool {
        // Since it's embedded, it's generally always healthy once initialized
        true
    }

    /// Wait for Meilisearch to become healthy
    pub async fn wait_for_health(&self, _timeout_secs: u64) -> bool {
        // Embedded engine is synchronous and ready when instantiated
        true
    }

    // ========================================================================
    // Index Management
    // ========================================================================

    /// Create or get an index
    pub async fn ensure_index(&self, name: &str, primary_key: Option<&str>) -> Result<()> {
        match self.meili.get_index(name) {
            Ok(idx) => {
                if let Some(pk) = primary_key {
                    let mut settings = self.meili.get_settings(name).map_err(|e| SearchError::MeilisearchError(e.to_string()))?;
                    // wait the embedded index engine does not allow changing primary_key on settings easily?
                    // Primary key is typically set on document addition or index creation in milli.
                    // If it exists, let's assume it's fine for now, or just update the index object wrapper itself.
                    // milli's Index doesn't store primary_key in standard `settings`. It's done via `UpdateIndexRequest`.
                    let req = crate::core::wilysearch::types::UpdateIndexRequest {
                        primary_key: pk.to_string(),
                    };
                    let _ = self.meili.update_index(name, &req);
                }
                Ok(())
            },
            Err(e) if matches!(e, WilySearchError::IndexNotFound(_)) => {
                let req = crate::core::wilysearch::types::CreateIndexRequest {
                    uid: name.to_string(),
                    primary_key: primary_key.map(|s| s.to_string()),
                };
                self.meili.create_index(&req)
                    .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;
                Ok(())
            }
            Err(e) => Err(SearchError::MeilisearchError(e.to_string())),
        }
    }

    /// Delete an index entirely
    pub async fn delete_index(&self, name: &str) -> Result<()> {
        match self.meili.delete_index(name) {
            Ok(_) => {
                log::info!("Deleted index '{}'", name);
                Ok(())
            },
            Err(e) if matches!(e, WilySearchError::IndexNotFound(_)) => {
                log::debug!("Index '{}' already doesn't exist", name);
                Ok(())
            }
            Err(e) => Err(SearchError::MeilisearchError(e.to_string())),
        }
    }

    /// Initialize all specialized indexes with appropriate settings
    pub async fn initialize_indexes(&self) -> Result<()> {
        log::info!("Initializing Meilisearch indexes (with embedded features)");

        // Create all content indexes
        for index_name in [INDEX_RULES, INDEX_FICTION, INDEX_CHAT, INDEX_DOCUMENTS] {
            self.ensure_index(index_name, Some("id")).await?;
        }

        // Create library metadata index
        self.ensure_index(INDEX_LIBRARY_METADATA, Some("id")).await?;

        // Apply settings to content indexes
        for index_name in [INDEX_RULES, INDEX_FICTION, INDEX_CHAT, INDEX_DOCUMENTS] {
            let mut settings = self.meili.get_settings(index_name)
                .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;

            settings.searchable_attributes = Some(vec![
                "content".to_string(), "source".to_string(), "metadata".to_string()
            ]);
            settings.filterable_attributes = Some(vec![
                "source".to_string(), "source_type".to_string(), "campaign_id".to_string(),
                "session_id".to_string(), "created_at".to_string()
            ].into_iter().collect());
            settings.sortable_attributes = Some(vec!["created_at".to_string()].into_iter().collect());

            self.meili.update_settings(index_name, &settings)
                .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;
        }

        // Configure library metadata index settings
        let mut library_settings = self.meili.get_settings(INDEX_LIBRARY_METADATA)
            .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;

        library_settings.searchable_attributes = Some(vec![
            "name".to_string(), "source_type".to_string(), "file_path".to_string(),
            // TTRPG metadata (searchable for discovery)
            "game_system".to_string(), "setting".to_string(), "content_type".to_string(), "publisher".to_string()
        ]);

        library_settings.filterable_attributes = Some(vec![
            "source_type".to_string(), "status".to_string(), "content_index".to_string(), "ingested_at".to_string(),
            // TTRPG metadata (filterable for organization)
            "game_system".to_string(), "setting".to_string(), "content_type".to_string(), "publisher".to_string()
        ].into_iter().collect());

        library_settings.sortable_attributes = Some(vec![
            "ingested_at".to_string(), "name".to_string(), "page_count".to_string(), "chunk_count".to_string()
        ].into_iter().collect());

        self.meili.update_settings(INDEX_LIBRARY_METADATA, &library_settings)
            .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;

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
        let mut settings = self.meili.get_settings(index_name)
            .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;

        let mut embedders = settings.embedders.unwrap_or_default();
        let wily_cfg: crate::core::wilysearch::types::EmbedderConfig = serde_json::from_value(build_embedder_json(config)).map_err(|e| SearchError::ConfigError(e.to_string()))?;
        embedders.insert(embedder_name.to_string(), wily_cfg);
        settings.embedders = Some(embedders);

        self.meili.update_settings(index_name, &settings)
            .map_err(|e| SearchError::ConfigError(e.to_string()))?;

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
        let settings = self.meili.get_settings(index_name)
            .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;

        if let Some(embedders) = settings.embedders {
            Ok(Some(serde_json::to_value(embedders).unwrap_or_default()))
        } else {
            Ok(None)
        }
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

        let json_docs: Vec<serde_json::Value> = documents
            .into_iter()
            .map(|doc| serde_json::to_value(doc).unwrap())
            .collect();

        let query = AddDocumentsQuery {
            primary_key: Some("id".to_string()),
            ..Default::default()
        };

        self.meili.add_or_replace_documents(index_name, &json_docs, &query)
            .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;

        log::info!(
            "Added {} documents to index '{}'",
            json_docs.len(),
            index_name
        );
        Ok(())
    }

    /// Delete documents by filter
    pub async fn delete_by_filter(&self, index_name: &str, filter: &str) -> Result<()> {
        let req = crate::core::wilysearch::types::DeleteDocumentsByFilterRequest {
            filter: filter.to_string(),
        };

        self.meili.delete_documents_by_filter(index_name, &req)
            .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;

        log::info!(
            "Deleted documents from index '{}' matching filter",
            index_name
        );
        Ok(())
    }

    /// Delete a document by ID
    pub async fn delete_document(&self, index_name: &str, doc_id: &str) -> Result<()> {
        self.meili.delete_document(index_name, doc_id)
            .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;
        Ok(())
    }

    /// Get document count for an index
    pub async fn document_count(&self, index_name: &str) -> Result<u64> {
        let stats = self.meili.index_stats(index_name)
            .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;
        Ok(stats.number_of_documents)
    }

    /// Clear all documents from an index
    pub async fn clear_index(&self, index_name: &str) -> Result<()> {
        self.meili.delete_all_documents(index_name)
            .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;

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
        let mut req = SearchRequest::default()
            .query(query)
            .limit(limit as u32);

        if let Some(f) = filter {
            req = req.filter(serde_json::json!(f));
        }

        let results = self.meili.search(index_name, &req)
            .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;

        let search_results: Vec<SearchResult> = results
            .hits
            .into_iter()
            .enumerate()
            .map(|(i, hit)| {
                let doc: SearchDocument = serde_json::from_value(hit).unwrap_or_default();
                SearchResult {
                    document: doc,
                    // Meilisearch doesn't give explicit scores in basic search
                    // Use position-based scoring
                    score: 1.0 - (i as f32 * 0.1).min(0.9),
                    index: index_name.to_string(),
                }
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
        let req = SearchRequest::default()
            .query(query)
            .limit(limit as u32)
            .hybrid(serde_json::json!({
                "semanticRatio": semantic_ratio,
                "embedder": embedder.unwrap_or("default")
            }));

        let response = match self.meili.search(index_name, &req) {
            Ok(res) => res,
            Err(_) => return self.search(index_name, query, limit, None).await, // Fallback to regular search
        };

        let search_results: Vec<SearchResult> = response
            .hits
            .into_iter()
            .enumerate()
            .map(|(i, hit)| {
                let doc: SearchDocument = serde_json::from_value(hit).unwrap_or_default();
                SearchResult {
                    document: doc,
                    score: 1.0 - (i as f32 * 0.1).min(0.9),
                    index: index_name.to_string(),
                }
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

    /// List all index names
    async fn list_all_index_names(&self) -> Result<Vec<String>> {
        let query = crate::core::wilysearch::types::PaginationQuery {
            offset: None,
            limit: Some(100), // milli indexes list gets all on the node realistically.
        };
        let index_list = self.meili.list_indexes(&query)
            .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;

        Ok(index_list.results.into_iter().map(|idx| idx.uid).collect())
    }

    // ========================================================================
    // Library Document Operations (delegated to LibraryRepositoryImpl)
    // ========================================================================

    pub fn library_repo(&self) -> LibraryRepositoryImpl {
        LibraryRepositoryImpl::new(&self.meili, "")
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

        let mut settings = self.meili.get_settings(index_name).map_err(|e| SearchError::MeilisearchError(e.to_string()))?;
        settings.searchable_attributes = Some(TTRPG_SEARCHABLE_ATTRIBUTES.iter().map(|s| s.to_string()).collect());
        settings.filterable_attributes = Some(TTRPG_FILTERABLE_ATTRIBUTES.iter().map(|s| s.to_string()).collect());
        settings.sortable_attributes = Some(TTRPG_SORTABLE_ATTRIBUTES.iter().map(|s| s.to_string()).collect());

        self.meili.update_settings(index_name, &settings)
            .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;

        log::info!("Configured TTRPG index '{}'", index_name);
        Ok(())
    }

    /// Configure a raw document index for the two-phase ingestion pipeline.
    /// Raw indexes store page-level documents and need sorting by page_number.
    pub async fn ensure_raw_index(&self, index_name: &str) -> Result<()> {
        // Create index if it doesn't exist
        self.ensure_index(index_name, Some("id")).await?;

        // Configure settings for raw document indexes
        let mut settings = self.meili.get_settings(index_name).map_err(|e| SearchError::MeilisearchError(e.to_string()))?;
        settings.searchable_attributes = Some(vec!["raw_content".to_string(), "source_slug".to_string()]);
        settings.sortable_attributes = Some(vec!["page_number".to_string()].into_iter().collect());

        self.meili.update_settings(index_name, &settings)
            .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;

        log::info!("Configured raw index '{}'", index_name);
        Ok(())
    }

    /// Configure a chunks index for the two-phase ingestion pipeline.
    /// Chunks indexes store semantic chunks with TTRPG metadata.
    pub async fn ensure_chunks_index(&self, index_name: &str) -> Result<()> {
        // Create index if it doesn't exist
        self.ensure_index(index_name, Some("id")).await?;

        // Configure settings for chunked document indexes with v2 enhanced metadata
        let mut settings = self.meili.get_settings(index_name).map_err(|e| SearchError::MeilisearchError(e.to_string()))?;
        settings.searchable_attributes = Some(vec![
            "content".to_string(),
            "embedding_content".to_string(), // Context-injected content for better semantic search
            "source_slug".to_string(),
            "book_title".to_string(),
            "game_system".to_string(),
            "setting".to_string(),
            "campaign_id".to_string(),
            "session_id".to_string(),
            "tags".to_string(),
            "semantic_keywords".to_string(),
        ]);
        settings.filterable_attributes = Some(vec![
            // v2 enhanced TTRPG filters
            "element_type".to_string(),     // stat_block, random_table, spell, item, etc.
            "content_mode".to_string(),     // crunch, fluff, mixed, example, optional, fiction
            "section_depth".to_string(),    // 0=root, 1=chapter, 2=section, etc.
            "parent_sections".to_string(),  // ["Chapter 1", "Monsters"]
            "cross_refs".to_string(),       // ["page:47", "chapter:3"]
            "dice_expressions".to_string(), // ["2d6", "1d20+5"]
            // Existing TTRPG metadata
            "game_system".to_string(),
            "game_system_id".to_string(),
            "content_category".to_string(), // rulebook, adventure, setting, bestiary
            "mechanic_type".to_string(),    // skill_check, combat, damage, etc.
            // Page/chunk tracking
            "page_start".to_string(),
            "page_end".to_string(),
            "source_slug".to_string(),
            "source".to_string(), // Filter/search by source document
        ].into_iter().collect());
        settings.sortable_attributes = Some(vec![
            "page_start".to_string(),
            "chunk_index".to_string(),
            "section_depth".to_string(),
            "classification_confidence".to_string(),
        ].into_iter().collect());

        self.meili.update_settings(index_name, &settings)
            .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;

        log::info!("Configured chunks index '{}'", index_name);
        Ok(())
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

        let json_docs: Vec<serde_json::Value> = documents
            .into_iter()
            .map(|doc| serde_json::to_value(doc).unwrap())
            .collect();

        let query = AddDocumentsQuery {
            primary_key: Some("id".to_string()),
            ..Default::default()
        };

        self.meili.add_or_replace_documents(index_name, &json_docs, &query)
            .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;

        log::info!(
            "Added {} TTRPG documents to index '{}'",
            json_docs.len(),
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
        let mut req = SearchRequest::default()
            .query(query)
            .limit(limit as u32);

        if let Some(f) = filter {
            req = req.filter(serde_json::json!(f));
        }

        let results = self.meili.search(index_name, &req)
            .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;

        let search_results: Vec<TTRPGSearchResult> = results
            .hits
            .into_iter()
            .enumerate()
            .filter_map(|(i, hit)| {
                let doc: TTRPGSearchDocument = serde_json::from_value(hit).ok()?;
                Some(TTRPGSearchResult {
                    document: doc,
                    score: 1.0 - (i as f32 * 0.1).min(0.9),
                    index: index_name.to_string(),
                })
            })
            .collect();

        Ok(search_results)
    }
}
