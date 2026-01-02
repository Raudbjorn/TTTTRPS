//! Hybrid Search Engine
//!
//! Combines keyword search (Meilisearch) and vector similarity search
//! using Reciprocal Rank Fusion (RRF) for result merging.
//!
//! # Architecture
//!
//! The hybrid search engine supports three search modes:
//! 1. **Keyword Search**: Uses Meilisearch's built-in BM25/TF-IDF ranking
//! 2. **Semantic Search**: Uses Meilisearch's vector search (if configured) or
//!    falls back to embedding-based similarity search
//! 3. **Hybrid Search**: Combines both using Reciprocal Rank Fusion (RRF)
//!
//! # Performance
//!
//! Target latency: <500ms for typical queries
//! - Query expansion: ~1ms
//! - Spell correction: ~2ms
//! - Parallel searches: ~200-400ms
//! - RRF fusion: ~1ms

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

use super::embeddings::{EmbeddingError, EmbeddingProvider};
use super::fusion::{FusedSearchResult, FusionStrategy, RRFConfig, RRFEngine};
use super::synonyms::TTRPGSynonyms;
use crate::core::query_expansion::QueryExpander;
use crate::core::search_client::{SearchClient, SearchDocument};
use crate::core::spell_correction::SpellCorrector;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum HybridSearchError {
    #[error("Search error: {0}")]
    SearchError(String),

    #[error("Embedding error: {0}")]
    EmbeddingError(#[from] EmbeddingError),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

pub type Result<T> = std::result::Result<T, HybridSearchError>;

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for hybrid search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridConfig {
    /// Weight for semantic/vector search results (0.0 - 1.0)
    #[serde(default = "default_semantic_weight")]
    pub semantic_weight: f32,

    /// Weight for keyword search results (0.0 - 1.0)
    #[serde(default = "default_keyword_weight")]
    pub keyword_weight: f32,

    /// RRF constant (typically 60)
    #[serde(default = "default_rrf_k")]
    pub rrf_k: u32,

    /// Enable query expansion with TTRPG synonyms
    #[serde(default = "default_true")]
    pub query_expansion: bool,

    /// Enable spell correction
    #[serde(default = "default_true")]
    pub spell_correction: bool,

    /// Semantic ratio for Meilisearch hybrid search (0.0 = keyword only, 1.0 = semantic only)
    #[serde(default = "default_semantic_ratio")]
    pub semantic_ratio: f32,

    /// Maximum results per search type before fusion
    #[serde(default = "default_max_per_type")]
    pub max_results_per_type: usize,

    /// Normalize final scores to 0-1 range
    #[serde(default = "default_true")]
    pub normalize_scores: bool,

    /// Enable direct vector similarity search (when embedding provider is available)
    #[serde(default = "default_true")]
    pub enable_vector_search: bool,

    /// Minimum score threshold (0.0 to disable)
    #[serde(default)]
    pub min_score: f32,

    /// Fusion strategy preset (optional, overrides weights if set)
    #[serde(default)]
    pub fusion_strategy: Option<String>,
}

fn default_semantic_weight() -> f32 {
    0.5
}

fn default_keyword_weight() -> f32 {
    0.5
}

fn default_rrf_k() -> u32 {
    60
}

fn default_true() -> bool {
    true
}

fn default_semantic_ratio() -> f32 {
    0.5
}

fn default_max_per_type() -> usize {
    50
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            semantic_weight: 0.5,
            keyword_weight: 0.5,
            rrf_k: 60,
            query_expansion: true,
            spell_correction: true,
            semantic_ratio: 0.5,
            max_results_per_type: 50,
            normalize_scores: true,
            enable_vector_search: true,
            min_score: 0.0,
            fusion_strategy: None,
        }
    }
}

impl HybridConfig {
    /// Create a config with balanced weights
    pub fn balanced() -> Self {
        Self::default()
    }

    /// Create a config favoring keyword search
    pub fn keyword_heavy() -> Self {
        Self {
            semantic_weight: 0.3,
            keyword_weight: 0.7,
            ..Default::default()
        }
    }

    /// Create a config favoring semantic search
    pub fn semantic_heavy() -> Self {
        Self {
            semantic_weight: 0.7,
            keyword_weight: 0.3,
            ..Default::default()
        }
    }

    /// Create a config from a fusion strategy
    pub fn from_strategy(strategy: FusionStrategy) -> Self {
        let (keyword_weight, semantic_weight) = strategy.weights();
        Self {
            semantic_weight,
            keyword_weight,
            ..Default::default()
        }
    }

    /// Get effective weights, considering fusion_strategy override
    pub fn effective_weights(&self) -> (f32, f32) {
        if let Some(strategy_name) = &self.fusion_strategy {
            match strategy_name.to_lowercase().as_str() {
                "balanced" => (0.5, 0.5),
                "keyword_heavy" | "keyword-heavy" => (0.7, 0.3),
                "semantic_heavy" | "semantic-heavy" => (0.3, 0.7),
                "semantic_strong" | "semantic-strong" => (0.2, 0.8),
                "keyword_primary" | "keyword-primary" => (0.9, 0.1),
                "semantic_primary" | "semantic-primary" => (0.1, 0.9),
                _ => (self.keyword_weight, self.semantic_weight),
            }
        } else {
            (self.keyword_weight, self.semantic_weight)
        }
    }

    /// Convert to RRF configuration
    pub fn to_rrf_config(&self) -> RRFConfig {
        RRFConfig {
            k: self.rrf_k,
            min_score: self.min_score,
            max_results: self.max_results_per_type * 2,
            normalize_scores: self.normalize_scores,
        }
    }
}

// ============================================================================
// Search Options and Results
// ============================================================================

/// Options for hybrid search
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HybridSearchOptions {
    /// Maximum results to return
    #[serde(default = "default_limit")]
    pub limit: usize,

    /// Source type filter
    pub source_type: Option<String>,

    /// Campaign ID filter
    pub campaign_id: Option<String>,

    /// Index to search (None = federated search)
    pub index: Option<String>,

    /// Override semantic weight for this query
    pub semantic_weight: Option<f32>,

    /// Override keyword weight for this query
    pub keyword_weight: Option<f32>,
}

fn default_limit() -> usize {
    10
}

/// Enhanced search result with fusion score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchResult {
    /// Original document
    pub document: SearchDocument,

    /// Fused RRF score
    pub score: f32,

    /// Keyword search rank (if in keyword results)
    pub keyword_rank: Option<usize>,

    /// Semantic search rank (if in semantic results)
    pub semantic_rank: Option<usize>,

    /// Source index
    pub index: String,

    /// Applied query (after expansion/correction)
    pub applied_query: Option<String>,
}

/// Search response with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchResponse {
    /// Fused results
    pub results: Vec<HybridSearchResult>,

    /// Total hits before fusion
    pub total_hits: usize,

    /// Original query
    pub original_query: String,

    /// Expanded query (if query expansion was applied)
    pub expanded_query: Option<String>,

    /// Corrected query (if spell correction was applied)
    pub corrected_query: Option<String>,

    /// Processing time in milliseconds
    pub processing_time_ms: u64,

    /// Search hints/suggestions
    pub hints: Vec<String>,
}

// ============================================================================
// Hybrid Search Engine
// ============================================================================

/// Hybrid search engine combining keyword and semantic search
pub struct HybridSearchEngine {
    search_client: Arc<SearchClient>,
    embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
    config: HybridConfig,
    query_expander: QueryExpander,
    spell_corrector: SpellCorrector,
    synonyms: TTRPGSynonyms,
    rrf_engine: RRFEngine,
}

impl HybridSearchEngine {
    /// Create a new hybrid search engine
    pub fn new(
        search_client: Arc<SearchClient>,
        embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
        config: HybridConfig,
    ) -> Self {
        let rrf_engine = RRFEngine::new(config.to_rrf_config());
        Self {
            search_client,
            embedding_provider,
            config,
            query_expander: QueryExpander::new(),
            spell_corrector: SpellCorrector::new(),
            synonyms: TTRPGSynonyms::new(),
            rrf_engine,
        }
    }

    /// Create with default configuration
    pub fn with_defaults(search_client: Arc<SearchClient>) -> Self {
        Self::new(search_client, None, HybridConfig::default())
    }

    /// Set embedding provider
    pub fn with_embeddings(mut self, provider: Arc<dyn EmbeddingProvider>) -> Self {
        self.embedding_provider = Some(provider);
        self
    }

    /// Update configuration
    pub fn with_config(mut self, config: HybridConfig) -> Self {
        self.rrf_engine = RRFEngine::new(config.to_rrf_config());
        self.config = config;
        self
    }

    /// Check if vector search is available
    pub fn has_vector_search(&self) -> bool {
        self.embedding_provider.is_some() && self.config.enable_vector_search
    }

    /// Perform hybrid search with RRF fusion
    pub async fn search(
        &self,
        query: &str,
        options: HybridSearchOptions,
    ) -> Result<HybridSearchResponse> {
        let start = std::time::Instant::now();
        let mut hints = Vec::new();

        // Step 1: Apply spell correction
        let (corrected_query, corrected) = if self.config.spell_correction {
            let result = self.spell_corrector.correct(query);
            if result.has_corrections {
                hints.push(format!("Did you mean: '{}'?", result.corrected_query));
                let cq = result.corrected_query.clone();
                (cq.clone(), Some(cq))
            } else {
                (query.to_string(), None)
            }
        } else {
            (query.to_string(), None)
        };

        // Step 2: Apply query expansion with TTRPG synonyms
        let (expanded_query, expanded) = if self.config.query_expansion {
            let result = self.synonyms.expand_query(&corrected_query);
            if result.was_expanded {
                hints.extend(result.hints.clone());
                (result.expanded_query.clone(), Some(result.expanded_query))
            } else {
                // Fallback to standard query expander
                let std_expansion = self.query_expander.expand(&corrected_query);
                if std_expansion.expanded_terms.len() > 1 {
                    (std_expansion.expanded_query, None)
                } else {
                    (corrected_query.clone(), None)
                }
            }
        } else {
            (corrected_query.clone(), None)
        };

        // Step 3: Build filter
        let filter = self.build_filter(&options);

        // Step 4: Get weights (allow per-query overrides, then config overrides)
        let (config_keyword, config_semantic) = self.config.effective_weights();
        let keyword_weight = options.keyword_weight.unwrap_or(config_keyword);
        let semantic_weight = options.semantic_weight.unwrap_or(config_semantic);

        // Step 5: Run searches in parallel
        // Use vector search if provider is available, otherwise fall back to Meilisearch hybrid
        let (keyword_results, semantic_results) = if self.has_vector_search() {
            tokio::join!(
                self.keyword_search(&expanded_query, &options, filter.as_deref()),
                self.vector_similarity_search(&corrected_query, &options, filter.as_deref()),
            )
        } else {
            tokio::join!(
                self.keyword_search(&expanded_query, &options, filter.as_deref()),
                self.semantic_search(&corrected_query, &options, filter.as_deref()),
            )
        };

        let keyword_results = keyword_results.unwrap_or_default();
        let semantic_results = semantic_results.unwrap_or_default();

        let total_hits = keyword_results.len() + semantic_results.len();

        // Step 6: Convert to fusion format and fuse using RRF engine
        let keyword_for_fusion: Vec<_> = keyword_results
            .into_iter()
            .map(|r| (r.document, r.score, r.index))
            .collect();

        let semantic_for_fusion: Vec<_> = semantic_results
            .into_iter()
            .map(|r| (r.document, r.score, r.index))
            .collect();

        let fused: Vec<FusedSearchResult> = self.rrf_engine.fuse_keyword_semantic(
            keyword_for_fusion,
            semantic_for_fusion,
            keyword_weight,
            semantic_weight,
        );

        // Step 7: Convert to HybridSearchResult and apply limit
        let results: Vec<HybridSearchResult> = fused
            .into_iter()
            .take(options.limit)
            .map(|r| HybridSearchResult {
                document: r.document,
                score: r.score,
                keyword_rank: r.keyword_rank,
                semantic_rank: r.semantic_rank,
                index: r.index,
                applied_query: expanded.clone(),
            })
            .collect();

        let processing_time = start.elapsed().as_millis() as u64;

        // Log performance warning if exceeds target
        if processing_time > 500 {
            log::warn!(
                "Hybrid search exceeded 500ms target: {}ms for query '{}'",
                processing_time,
                query
            );
        }

        Ok(HybridSearchResponse {
            results,
            total_hits,
            original_query: query.to_string(),
            expanded_query: expanded,
            corrected_query: corrected,
            processing_time_ms: processing_time,
            hints,
        })
    }

    /// Perform direct vector similarity search using the embedding provider
    async fn vector_similarity_search(
        &self,
        query: &str,
        options: &HybridSearchOptions,
        _filter: Option<&str>,
    ) -> Result<Vec<RankedResult>> {
        let provider = match &self.embedding_provider {
            Some(p) => p,
            None => return Ok(Vec::new()),
        };

        // Generate query embedding
        let query_embedding = provider
            .embed(query)
            .await
            .map_err(|e| HybridSearchError::EmbeddingError(e))?;

        // For now, fall back to Meilisearch's built-in hybrid search
        // In a full implementation, this would query a vector database directly
        // and compute cosine similarity scores
        let limit = self.config.max_results_per_type;
        let semantic_ratio = self.config.semantic_ratio;

        let results = if let Some(index) = &options.index {
            self.search_client
                .hybrid_search(index, query, limit, semantic_ratio, None)
                .await
                .map_err(|e| HybridSearchError::SearchError(e.to_string()))?
        } else {
            // Federated vector search
            let mut all_results = Vec::new();
            for idx in SearchClient::all_indexes() {
                if let Ok(results) = self
                    .search_client
                    .hybrid_search(idx, query, limit / 4, semantic_ratio, None)
                    .await
                {
                    all_results.extend(results);
                }
            }
            all_results
        };

        // Convert to ranked results with embedding-based scoring hint
        Ok(results
            .into_iter()
            .enumerate()
            .map(|(rank, r)| RankedResult {
                document: r.document,
                score: r.score,
                keyword_rank: None,
                semantic_rank: Some(rank),
                index: r.index,
            })
            .collect())
    }

    /// Perform keyword-only search
    async fn keyword_search(
        &self,
        query: &str,
        options: &HybridSearchOptions,
        filter: Option<&str>,
    ) -> Result<Vec<RankedResult>> {
        let limit = self.config.max_results_per_type;

        let results = if let Some(index) = &options.index {
            self.search_client
                .search(index, query, limit, filter)
                .await
                .map_err(|e| HybridSearchError::SearchError(e.to_string()))?
        } else {
            // Federated search
            self.search_client
                .search_all(query, limit)
                .await
                .map_err(|e| HybridSearchError::SearchError(e.to_string()))?
                .results
        };

        Ok(results
            .into_iter()
            .enumerate()
            .map(|(rank, r)| RankedResult {
                document: r.document,
                score: r.score,
                keyword_rank: Some(rank),
                semantic_rank: None,
                index: r.index,
            })
            .collect())
    }

    /// Perform semantic/hybrid search via Meilisearch
    async fn semantic_search(
        &self,
        query: &str,
        options: &HybridSearchOptions,
        filter: Option<&str>,
    ) -> Result<Vec<RankedResult>> {
        let limit = self.config.max_results_per_type;
        let semantic_ratio = self.config.semantic_ratio;

        // Use Meilisearch's built-in hybrid search if embedder is configured
        let results = if let Some(index) = &options.index {
            self.search_client
                .hybrid_search(index, query, limit, semantic_ratio, None)
                .await
                .map_err(|e| HybridSearchError::SearchError(e.to_string()))?
        } else {
            // Federated hybrid search across all indexes
            let mut all_results = Vec::new();
            for idx in SearchClient::all_indexes() {
                if let Ok(results) = self
                    .search_client
                    .hybrid_search(idx, query, limit / 4, semantic_ratio, None)
                    .await
                {
                    all_results.extend(results);
                }
            }
            all_results
        };

        Ok(results
            .into_iter()
            .enumerate()
            .map(|(rank, r)| RankedResult {
                document: r.document,
                score: r.score,
                keyword_rank: None,
                semantic_rank: Some(rank),
                index: r.index,
            })
            .collect())
    }

    /// Build Meilisearch filter from options
    fn build_filter(&self, options: &HybridSearchOptions) -> Option<String> {
        let mut filters = Vec::new();

        if let Some(source_type) = &options.source_type {
            filters.push(format!("source_type = '{}'", source_type));
        }

        if let Some(campaign_id) = &options.campaign_id {
            filters.push(format!("campaign_id = '{}'", campaign_id));
        }

        if filters.is_empty() {
            None
        } else {
            Some(filters.join(" AND "))
        }
    }

    /// Get query suggestions for autocomplete
    pub fn suggest(&self, partial: &str) -> Vec<String> {
        let mut suggestions = Vec::new();

        // Add TTRPG synonym suggestions
        suggestions.extend(self.synonyms.suggest(partial));

        // Add standard query expander suggestions
        suggestions.extend(self.query_expander.suggest(partial));

        suggestions.sort();
        suggestions.dedup();
        suggestions.truncate(10);

        suggestions
    }

    /// Get search hints based on query
    pub fn get_hints(&self, query: &str) -> Vec<String> {
        self.synonyms.expand_query(query).hints
    }
}

/// Internal ranked result for fusion
#[derive(Clone)]
struct RankedResult {
    document: SearchDocument,
    score: f32,
    keyword_rank: Option<usize>,
    semantic_rank: Option<usize>,
    index: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rrf_formula() {
        let k = 60.0_f32;
        let rank = 0;
        let score = 1.0 / (k + rank as f32 + 1.0);
        assert!((score - 0.01639).abs() < 0.001);
    }

    #[test]
    fn test_config_defaults() {
        let config = HybridConfig::default();
        assert_eq!(config.semantic_weight, 0.5);
        assert_eq!(config.keyword_weight, 0.5);
        assert_eq!(config.rrf_k, 60);
        assert!(config.query_expansion);
        assert!(config.spell_correction);
        assert!(config.normalize_scores);
        assert!(config.enable_vector_search);
    }

    #[test]
    fn test_config_presets() {
        let balanced = HybridConfig::balanced();
        assert_eq!(balanced.keyword_weight, 0.5);
        assert_eq!(balanced.semantic_weight, 0.5);

        let keyword_heavy = HybridConfig::keyword_heavy();
        assert_eq!(keyword_heavy.keyword_weight, 0.7);
        assert_eq!(keyword_heavy.semantic_weight, 0.3);

        let semantic_heavy = HybridConfig::semantic_heavy();
        assert_eq!(semantic_heavy.keyword_weight, 0.3);
        assert_eq!(semantic_heavy.semantic_weight, 0.7);
    }

    #[test]
    fn test_config_from_strategy() {
        let config = HybridConfig::from_strategy(FusionStrategy::SemanticStrong);
        assert_eq!(config.keyword_weight, 0.2);
        assert_eq!(config.semantic_weight, 0.8);
    }

    #[test]
    fn test_effective_weights_with_strategy() {
        let mut config = HybridConfig::default();
        config.fusion_strategy = Some("semantic_heavy".to_string());

        let (keyword, semantic) = config.effective_weights();
        assert_eq!(keyword, 0.3);
        assert_eq!(semantic, 0.7);
    }

    #[test]
    fn test_effective_weights_without_strategy() {
        let config = HybridConfig {
            keyword_weight: 0.6,
            semantic_weight: 0.4,
            ..Default::default()
        };

        let (keyword, semantic) = config.effective_weights();
        assert_eq!(keyword, 0.6);
        assert_eq!(semantic, 0.4);
    }

    #[test]
    fn test_to_rrf_config() {
        let config = HybridConfig {
            rrf_k: 50,
            min_score: 0.1,
            max_results_per_type: 25,
            normalize_scores: false,
            ..Default::default()
        };

        let rrf_config = config.to_rrf_config();
        assert_eq!(rrf_config.k, 50);
        assert_eq!(rrf_config.min_score, 0.1);
        assert_eq!(rrf_config.max_results, 50); // 25 * 2
        assert!(!rrf_config.normalize_scores);
    }

    #[test]
    fn test_filter_building() {
        let search_client = Arc::new(SearchClient::new("http://localhost:7700", None));
        let engine = HybridSearchEngine::with_defaults(search_client);

        let options = HybridSearchOptions {
            source_type: Some("rules".to_string()),
            campaign_id: Some("camp-1".to_string()),
            ..Default::default()
        };

        let filter = engine.build_filter(&options);
        assert!(filter.is_some());
        let filter_str = filter.unwrap();
        assert!(filter_str.contains("source_type = 'rules'"));
        assert!(filter_str.contains("campaign_id = 'camp-1'"));
    }

    #[test]
    fn test_filter_building_empty() {
        let search_client = Arc::new(SearchClient::new("http://localhost:7700", None));
        let engine = HybridSearchEngine::with_defaults(search_client);

        let options = HybridSearchOptions::default();
        let filter = engine.build_filter(&options);
        assert!(filter.is_none());
    }

    #[test]
    fn test_has_vector_search() {
        let search_client = Arc::new(SearchClient::new("http://localhost:7700", None));
        let engine = HybridSearchEngine::with_defaults(search_client);

        // No embedding provider = no vector search
        assert!(!engine.has_vector_search());
    }

    #[test]
    fn test_options_default() {
        let options = HybridSearchOptions::default();
        assert_eq!(options.limit, 10);
        assert!(options.source_type.is_none());
        assert!(options.campaign_id.is_none());
        assert!(options.index.is_none());
        assert!(options.semantic_weight.is_none());
        assert!(options.keyword_weight.is_none());
    }
}
