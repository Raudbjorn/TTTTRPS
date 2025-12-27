//! Hybrid Search Module
//!
//! Combines semantic (vector) search with keyword (BM25) search for improved
//! retrieval accuracy. Uses Reciprocal Rank Fusion (RRF) to merge results.

use crate::core::vector_store::{VectorStore, SearchResult as VectorSearchResult, VectorStoreError};
use crate::core::keyword_search::{KeywordIndex, KeywordSearchResult, KeywordSearchError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum HybridSearchError {
    #[error("Vector store error: {0}")]
    VectorError(#[from] VectorStoreError),

    #[error("Keyword search error: {0}")]
    KeywordError(#[from] KeywordSearchError),

    #[error("No search backend available")]
    NoBackendAvailable,
}

pub type Result<T> = std::result::Result<T, HybridSearchError>;

// ============================================================================
// Search Result Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchResult {
    pub id: String,
    pub content: String,
    pub source: String,
    pub source_type: String,
    pub chunk_index: i32,
    /// Combined score from both search methods
    pub score: f32,
    /// Individual scores for transparency
    pub vector_score: Option<f32>,
    pub keyword_score: Option<f32>,
    /// Rank in each result set
    pub vector_rank: Option<usize>,
    pub keyword_rank: Option<usize>,
}

// ============================================================================
// Search Configuration
// ============================================================================

#[derive(Debug, Clone)]
pub struct HybridSearchConfig {
    /// Weight for vector search results (0.0 - 1.0)
    pub vector_weight: f32,
    /// Weight for keyword search results (0.0 - 1.0)
    pub keyword_weight: f32,
    /// RRF constant (typically 60)
    pub rrf_k: f32,
    /// Whether to use RRF (true) or weighted linear combination (false)
    pub use_rrf: bool,
}

impl Default for HybridSearchConfig {
    fn default() -> Self {
        Self {
            vector_weight: 0.7,
            keyword_weight: 0.3,
            rrf_k: 60.0,
            use_rrf: true,
        }
    }
}

// ============================================================================
// Hybrid Search Engine
// ============================================================================

/// Hybrid search combining semantic and keyword search
pub struct HybridSearchEngine<'a> {
    vector_store: &'a VectorStore,
    keyword_index: &'a KeywordIndex,
    config: HybridSearchConfig,
}

impl<'a> HybridSearchEngine<'a> {
    /// Create a new hybrid search engine
    pub fn new(
        vector_store: &'a VectorStore,
        keyword_index: &'a KeywordIndex,
        config: HybridSearchConfig,
    ) -> Self {
        Self {
            vector_store,
            keyword_index,
            config,
        }
    }

    /// Create with default configuration
    pub fn with_defaults(vector_store: &'a VectorStore, keyword_index: &'a KeywordIndex) -> Self {
        Self::new(vector_store, keyword_index, HybridSearchConfig::default())
    }

    /// Perform hybrid search
    pub async fn search(
        &self,
        query: &str,
        query_embedding: &[f32],
        limit: usize,
        source_filter: Option<&str>,
    ) -> Result<Vec<HybridSearchResult>> {
        // Fetch more results from each source to allow for good fusion
        let fetch_limit = limit * 3;

        // Perform both searches concurrently
        let vector_results = self
            .vector_store
            .search(query_embedding, fetch_limit, source_filter)
            .await?;

        let keyword_results = self
            .keyword_index
            .search_with_filter(query, source_filter, fetch_limit)?;

        // Fuse results
        let fused = if self.config.use_rrf {
            self.reciprocal_rank_fusion(vector_results, keyword_results)
        } else {
            self.weighted_combination(vector_results, keyword_results)
        };

        // Sort by score and limit
        let mut results: Vec<_> = fused.into_values().collect();
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);

        Ok(results)
    }

    /// Perform vector-only search
    pub async fn search_semantic(
        &self,
        query_embedding: &[f32],
        limit: usize,
        source_filter: Option<&str>,
    ) -> Result<Vec<HybridSearchResult>> {
        let results = self
            .vector_store
            .search(query_embedding, limit, source_filter)
            .await?;

        Ok(results
            .into_iter()
            .enumerate()
            .map(|(rank, r)| HybridSearchResult {
                id: r.document.id,
                content: r.document.content,
                source: r.document.source,
                source_type: r.document.source_type,
                chunk_index: r.document.chunk_index,
                score: r.score,
                vector_score: Some(r.score),
                keyword_score: None,
                vector_rank: Some(rank + 1),
                keyword_rank: None,
            })
            .collect())
    }

    /// Perform keyword-only search
    pub fn search_keyword(
        &self,
        query: &str,
        limit: usize,
        source_filter: Option<&str>,
    ) -> Result<Vec<HybridSearchResult>> {
        let results = self
            .keyword_index
            .search_with_filter(query, source_filter, limit)?;

        Ok(results
            .into_iter()
            .enumerate()
            .map(|(rank, r)| HybridSearchResult {
                id: r.id,
                content: r.content,
                source: r.source,
                source_type: r.source_type,
                chunk_index: r.chunk_index,
                score: r.score,
                vector_score: None,
                keyword_score: Some(r.score),
                vector_rank: None,
                keyword_rank: Some(rank + 1),
            })
            .collect())
    }

    /// Reciprocal Rank Fusion for combining results
    /// RRF score = sum(1 / (k + rank)) for each result set
    fn reciprocal_rank_fusion(
        &self,
        vector_results: Vec<VectorSearchResult>,
        keyword_results: Vec<KeywordSearchResult>,
    ) -> HashMap<String, HybridSearchResult> {
        let mut fused: HashMap<String, HybridSearchResult> = HashMap::new();
        let k = self.config.rrf_k;

        // Process vector results
        for (rank, result) in vector_results.into_iter().enumerate() {
            let rrf_score = 1.0 / (k + (rank + 1) as f32);

            let entry = fused.entry(result.document.id.clone()).or_insert(HybridSearchResult {
                id: result.document.id,
                content: result.document.content,
                source: result.document.source,
                source_type: result.document.source_type,
                chunk_index: result.document.chunk_index,
                score: 0.0,
                vector_score: None,
                keyword_score: None,
                vector_rank: None,
                keyword_rank: None,
            });

            entry.score += rrf_score * self.config.vector_weight;
            entry.vector_score = Some(result.score);
            entry.vector_rank = Some(rank + 1);
        }

        // Process keyword results
        for (rank, result) in keyword_results.into_iter().enumerate() {
            let rrf_score = 1.0 / (k + (rank + 1) as f32);

            let entry = fused.entry(result.id.clone()).or_insert(HybridSearchResult {
                id: result.id,
                content: result.content,
                source: result.source,
                source_type: result.source_type,
                chunk_index: result.chunk_index,
                score: 0.0,
                vector_score: None,
                keyword_score: None,
                vector_rank: None,
                keyword_rank: None,
            });

            entry.score += rrf_score * self.config.keyword_weight;
            entry.keyword_score = Some(result.score);
            entry.keyword_rank = Some(rank + 1);
        }

        fused
    }

    /// Weighted linear combination of normalized scores
    fn weighted_combination(
        &self,
        vector_results: Vec<VectorSearchResult>,
        keyword_results: Vec<KeywordSearchResult>,
    ) -> HashMap<String, HybridSearchResult> {
        let mut fused: HashMap<String, HybridSearchResult> = HashMap::new();

        // Find max scores for normalization
        let max_vector_score = vector_results
            .iter()
            .map(|r| r.score)
            .fold(0.0f32, |a, b| a.max(b));
        let max_keyword_score = keyword_results
            .iter()
            .map(|r| r.score)
            .fold(0.0f32, |a, b| a.max(b));

        // Process vector results
        for (rank, result) in vector_results.into_iter().enumerate() {
            let normalized_score = if max_vector_score > 0.0 {
                result.score / max_vector_score
            } else {
                0.0
            };

            let entry = fused.entry(result.document.id.clone()).or_insert(HybridSearchResult {
                id: result.document.id,
                content: result.document.content,
                source: result.document.source,
                source_type: result.document.source_type,
                chunk_index: result.document.chunk_index,
                score: 0.0,
                vector_score: None,
                keyword_score: None,
                vector_rank: None,
                keyword_rank: None,
            });

            entry.score += normalized_score * self.config.vector_weight;
            entry.vector_score = Some(result.score);
            entry.vector_rank = Some(rank + 1);
        }

        // Process keyword results
        for (rank, result) in keyword_results.into_iter().enumerate() {
            let normalized_score = if max_keyword_score > 0.0 {
                result.score / max_keyword_score
            } else {
                0.0
            };

            let entry = fused.entry(result.id.clone()).or_insert(HybridSearchResult {
                id: result.id,
                content: result.content,
                source: result.source,
                source_type: result.source_type,
                chunk_index: result.chunk_index,
                score: 0.0,
                vector_score: None,
                keyword_score: None,
                vector_rank: None,
                keyword_rank: None,
            });

            entry.score += normalized_score * self.config.keyword_weight;
            entry.keyword_score = Some(result.score);
            entry.keyword_rank = Some(rank + 1);
        }

        fused
    }
}

// ============================================================================
// Standalone Fusion Functions
// ============================================================================

/// Perform RRF fusion on pre-computed results without stores
pub fn fuse_results_rrf(
    vector_results: Vec<(String, f32)>, // (id, score)
    keyword_results: Vec<(String, f32)>,
    k: f32,
    vector_weight: f32,
    keyword_weight: f32,
) -> Vec<(String, f32)> {
    let mut scores: HashMap<String, f32> = HashMap::new();

    for (rank, (id, _)) in vector_results.into_iter().enumerate() {
        let rrf = 1.0 / (k + (rank + 1) as f32);
        *scores.entry(id).or_insert(0.0) += rrf * vector_weight;
    }

    for (rank, (id, _)) in keyword_results.into_iter().enumerate() {
        let rrf = 1.0 / (k + (rank + 1) as f32);
        *scores.entry(id).or_insert(0.0) += rrf * keyword_weight;
    }

    let mut results: Vec<_> = scores.into_iter().collect();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rrf_fusion() {
        let vector = vec![
            ("doc-1".to_string(), 0.9),
            ("doc-2".to_string(), 0.8),
            ("doc-3".to_string(), 0.7),
        ];

        let keyword = vec![
            ("doc-2".to_string(), 5.0),
            ("doc-1".to_string(), 4.0),
            ("doc-4".to_string(), 3.0),
        ];

        let results = fuse_results_rrf(vector, keyword, 60.0, 0.7, 0.3);

        // doc-2 should rank highest (appears in both lists with good ranks)
        assert!(!results.is_empty());
        // First two should be doc-1 and doc-2 (in either order)
        let top_ids: Vec<_> = results.iter().take(2).map(|(id, _)| id.clone()).collect();
        assert!(top_ids.contains(&"doc-1".to_string()));
        assert!(top_ids.contains(&"doc-2".to_string()));
    }

    #[test]
    fn test_config_defaults() {
        let config = HybridSearchConfig::default();
        assert_eq!(config.vector_weight, 0.7);
        assert_eq!(config.keyword_weight, 0.3);
        assert!(config.use_rrf);
    }
}
