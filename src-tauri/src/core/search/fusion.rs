//! Reciprocal Rank Fusion Module
//!
//! Implements the RRF algorithm for combining results from multiple search methods.
//! RRF is particularly effective for hybrid search as it handles different score
//! distributions gracefully.
//!
//! # Algorithm
//!
//! The RRF score for a document d across multiple result sets is:
//!
//! ```text
//! RRF(d) = sum over all result sets: weight_i / (k + rank_i(d))
//! ```
//!
//! where:
//! - k is a constant (typically 60) that dampens the influence of high rankings
//! - rank_i(d) is the rank of document d in result set i (1-indexed)
//! - weight_i is the weight assigned to result set i
//!
//! Documents appearing in multiple result sets accumulate higher scores,
//! making RRF naturally boost cross-method agreement.
//!
//! # Configuration
//!
//! Default parameters are derived from the MDMAI codebase via `fusion_config` in the
//! vocabulary module, providing RAG-optimized defaults.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::core::search_client::SearchDocument;
use crate::ingestion::ttrpg::vocabulary::fusion_config;

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for Reciprocal Rank Fusion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RRFConfig {
    /// RRF constant (k parameter). Higher values reduce the impact of rank position.
    /// Typical values range from 50 to 70, with 60 being the most common.
    /// Default: 60 (from fusion_config::RRF_K)
    #[serde(default = "default_k")]
    pub k: u32,

    /// Minimum score threshold - results below this are excluded.
    /// Default: 0.1 (from fusion_config::MIN_SCORE)
    #[serde(default = "default_min_score")]
    pub min_score: f32,

    /// Maximum number of results to return after fusion.
    /// Default: 20 (from fusion_config::MAX_RESULTS)
    #[serde(default = "default_max_results")]
    pub max_results: usize,

    /// Whether to normalize final scores to 0-1 range
    #[serde(default = "default_true")]
    pub normalize_scores: bool,
}

fn default_k() -> u32 {
    fusion_config::RRF_K as u32
}

fn default_min_score() -> f32 {
    fusion_config::MIN_SCORE
}

fn default_max_results() -> usize {
    fusion_config::MAX_RESULTS
}

fn default_true() -> bool {
    true
}

impl Default for RRFConfig {
    /// Default configuration using MDMAI-derived constants from fusion_config.
    ///
    /// Values:
    /// - k: 60 (RRF constant)
    /// - min_score: 0.1 (minimum score threshold)
    /// - max_results: 20 (maximum results to return)
    fn default() -> Self {
        Self {
            k: fusion_config::RRF_K as u32,
            min_score: fusion_config::MIN_SCORE,
            max_results: fusion_config::MAX_RESULTS,
            normalize_scores: true,
        }
    }
}

impl RRFConfig {
    /// Create a lenient configuration with no minimum score and more results.
    /// Good for exploration or when you want to see lower-ranked results.
    pub fn lenient() -> Self {
        Self {
            k: fusion_config::RRF_K as u32,
            min_score: 0.0,
            max_results: fusion_config::MAX_RESULTS * 5, // 100 results
            normalize_scores: true,
        }
    }

    /// Create a strict configuration with higher minimum score threshold.
    /// Good for precision-focused searches where only top matches matter.
    pub fn strict() -> Self {
        Self {
            k: fusion_config::RRF_K as u32,
            min_score: fusion_config::MIN_SCORE * 2.0, // 0.2
            max_results: fusion_config::MAX_RESULTS / 2, // 10 results
            normalize_scores: true,
        }
    }
}

// ============================================================================
// Ranked Result Types
// ============================================================================

/// A search result with ranking information from a specific source
#[derive(Debug, Clone)]
pub struct RankedItem<T> {
    /// The item being ranked
    pub item: T,
    /// Original rank in the source result set (0-indexed)
    pub rank: usize,
    /// Original score from the source (if available)
    pub original_score: Option<f32>,
    /// Source identifier (e.g., "keyword", "semantic", "vector")
    pub source: String,
}

/// A fused result after RRF
#[derive(Debug, Clone)]
pub struct FusedResult<T> {
    /// The fused item
    pub item: T,
    /// Final RRF score
    pub score: f32,
    /// Ranks from each source (source_name -> rank)
    pub source_ranks: HashMap<String, usize>,
    /// Number of sources that contained this result
    pub source_count: usize,
}

/// Search result specifically for document fusion
#[derive(Debug, Clone)]
pub struct FusedSearchResult {
    /// The document
    pub document: SearchDocument,
    /// Fused RRF score
    pub score: f32,
    /// Keyword search rank (if present)
    pub keyword_rank: Option<usize>,
    /// Semantic/vector search rank (if present)
    pub semantic_rank: Option<usize>,
    /// Source index name
    pub index: String,
    /// Number of search methods that found this result
    pub overlap_count: usize,
}

// ============================================================================
// Reciprocal Rank Fusion Engine
// ============================================================================

/// Reciprocal Rank Fusion engine for combining multiple ranked result sets
pub struct RRFEngine {
    config: RRFConfig,
}

impl RRFEngine {
    /// Create a new RRF engine with the given configuration
    pub fn new(config: RRFConfig) -> Self {
        Self { config }
    }

    /// Create an RRF engine with default configuration
    pub fn with_defaults() -> Self {
        Self::new(RRFConfig::default())
    }

    /// Create an RRF engine with a custom k value
    pub fn with_k(k: u32) -> Self {
        Self::new(RRFConfig {
            k,
            ..Default::default()
        })
    }

    /// Fuse multiple ranked result sets using RRF
    ///
    /// # Type Parameters
    /// - `T`: The item type being ranked
    /// - `K`: Key type for deduplication (must implement Hash + Eq)
    ///
    /// # Arguments
    /// - `result_sets`: Vec of (weight, results) tuples. Weight should be in [0, 1].
    /// - `key_fn`: Function to extract a unique key from each item for deduplication
    ///
    /// # Returns
    /// A vector of fused results sorted by RRF score (descending)
    pub fn fuse<T, K, F>(
        &self,
        result_sets: Vec<(f32, Vec<RankedItem<T>>)>,
        key_fn: F,
    ) -> Vec<FusedResult<T>>
    where
        T: Clone,
        K: std::hash::Hash + Eq,
        F: Fn(&T) -> K,
    {
        let k = self.config.k as f32;
        let mut scores: HashMap<K, FusedResult<T>> = HashMap::new();

        // Process each result set
        for (weight, results) in result_sets {
            for item in results {
                let key = key_fn(&item.item);
                // RRF formula: weight / (k + rank + 1)
                // We use rank + 1 because ranks are 0-indexed but RRF uses 1-indexed
                let rrf_contribution = weight / (k + item.rank as f32 + 1.0);

                scores
                    .entry(key)
                    .and_modify(|e| {
                        e.score += rrf_contribution;
                        e.source_ranks.insert(item.source.clone(), item.rank);
                        e.source_count += 1;
                    })
                    .or_insert_with(|| {
                        let mut source_ranks = HashMap::new();
                        source_ranks.insert(item.source.clone(), item.rank);
                        FusedResult {
                            item: item.item.clone(),
                            score: rrf_contribution,
                            source_ranks,
                            source_count: 1,
                        }
                    });
            }
        }

        // Collect and sort by score
        let mut results: Vec<FusedResult<T>> = scores.into_values().collect();
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply minimum score threshold
        if self.config.min_score > 0.0 {
            results.retain(|r| r.score >= self.config.min_score);
        }

        // Normalize scores if configured
        if self.config.normalize_scores && !results.is_empty() {
            let max_score = results[0].score;
            if max_score > 0.0 {
                for result in &mut results {
                    result.score /= max_score;
                }
            }
        }

        // Apply max results limit
        results.truncate(self.config.max_results);

        results
    }

    /// Convenience method to fuse keyword and semantic search results
    ///
    /// This is the primary fusion method for hybrid search, combining
    /// keyword (BM25/TF-IDF) and semantic (vector similarity) results.
    pub fn fuse_keyword_semantic(
        &self,
        keyword_results: Vec<(SearchDocument, f32, String)>,
        semantic_results: Vec<(SearchDocument, f32, String)>,
        keyword_weight: f32,
        semantic_weight: f32,
    ) -> Vec<FusedSearchResult> {
        let k = self.config.k as f32;
        let mut scores: HashMap<String, FusedSearchResult> = HashMap::new();

        // Process keyword results
        for (rank, (doc, _score, index)) in keyword_results.into_iter().enumerate() {
            let doc_id = doc.id.clone();
            let rrf_score = keyword_weight / (k + rank as f32 + 1.0);

            scores
                .entry(doc_id)
                .and_modify(|e| {
                    e.score += rrf_score;
                    e.keyword_rank = Some(rank);
                    e.overlap_count += 1;
                })
                .or_insert_with(|| FusedSearchResult {
                    document: doc,
                    score: rrf_score,
                    keyword_rank: Some(rank),
                    semantic_rank: None,
                    index,
                    overlap_count: 1,
                });
        }

        // Process semantic results
        for (rank, (doc, _score, index)) in semantic_results.into_iter().enumerate() {
            let doc_id = doc.id.clone();
            let rrf_score = semantic_weight / (k + rank as f32 + 1.0);

            scores
                .entry(doc_id)
                .and_modify(|e| {
                    e.score += rrf_score;
                    e.semantic_rank = Some(rank);
                    e.overlap_count += 1;
                })
                .or_insert_with(|| FusedSearchResult {
                    document: doc,
                    score: rrf_score,
                    keyword_rank: None,
                    semantic_rank: Some(rank),
                    index,
                    overlap_count: 1,
                });
        }

        // Collect and sort
        let mut results: Vec<FusedSearchResult> = scores.into_values().collect();
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply minimum score threshold
        if self.config.min_score > 0.0 {
            results.retain(|r| r.score >= self.config.min_score);
        }

        // Normalize scores
        if self.config.normalize_scores && !results.is_empty() {
            let max_score = results[0].score;
            if max_score > 0.0 {
                for result in &mut results {
                    result.score /= max_score;
                }
            }
        }

        // Apply limit
        results.truncate(self.config.max_results);

        results
    }

    /// Calculate the theoretical maximum RRF score for a result
    ///
    /// This occurs when a result is ranked #1 in all result sets.
    /// Useful for understanding score normalization.
    pub fn max_theoretical_score(&self, weights: &[f32]) -> f32 {
        let k = self.config.k as f32;
        weights.iter().map(|w| w / (k + 1.0)).sum()
    }

    /// Calculate the RRF contribution for a specific rank
    pub fn score_at_rank(&self, rank: usize, weight: f32) -> f32 {
        let k = self.config.k as f32;
        weight / (k + rank as f32 + 1.0)
    }
}

// ============================================================================
// Weighted Fusion Strategies
// ============================================================================

/// Predefined weight configurations for different use cases
#[derive(Debug, Clone, Copy)]
pub enum FusionStrategy {
    /// Equal weight to keyword and semantic (0.5, 0.5)
    Balanced,
    /// Favor keyword search (0.7, 0.3)
    KeywordHeavy,
    /// Favor semantic search (0.3, 0.7)
    SemanticHeavy,
    /// Strong semantic preference (0.2, 0.8)
    SemanticStrong,
    /// Keyword only - semantic as tiebreaker (0.9, 0.1)
    KeywordPrimary,
    /// Semantic only - keyword as tiebreaker (0.1, 0.9)
    SemanticPrimary,
    /// MDMAI vocabulary config weights (BM25: 0.4, Vector: 0.6)
    /// Optimized for TTRPG RAG based on empirical testing
    VocabularyOptimized,
    /// Custom weights
    Custom(f32, f32),
}

impl FusionStrategy {
    /// Get the (keyword_weight, semantic_weight) tuple
    pub fn weights(&self) -> (f32, f32) {
        match self {
            FusionStrategy::Balanced => (0.5, 0.5),
            FusionStrategy::KeywordHeavy => (0.7, 0.3),
            FusionStrategy::SemanticHeavy => (0.3, 0.7),
            FusionStrategy::SemanticStrong => (0.2, 0.8),
            FusionStrategy::KeywordPrimary => (0.9, 0.1),
            FusionStrategy::SemanticPrimary => (0.1, 0.9),
            FusionStrategy::VocabularyOptimized => (
                fusion_config::BM25_WEIGHT,
                fusion_config::VECTOR_WEIGHT,
            ),
            FusionStrategy::Custom(k, s) => (*k, *s),
        }
    }

    /// Create a strategy from explicit weights
    pub fn from_weights(keyword_weight: f32, semantic_weight: f32) -> Self {
        // Normalize weights
        let total = keyword_weight + semantic_weight;
        if total > 0.0 {
            FusionStrategy::Custom(keyword_weight / total, semantic_weight / total)
        } else {
            FusionStrategy::Balanced
        }
    }

    /// Get the MDMAI-derived optimal weights for TTRPG content
    /// Returns (BM25_WEIGHT, VECTOR_WEIGHT) from vocabulary config
    pub fn vocabulary_weights() -> (f32, f32) {
        (fusion_config::BM25_WEIGHT, fusion_config::VECTOR_WEIGHT)
    }

    /// Get boost factors for special matches
    pub fn boost_factors() -> (f32, f32) {
        (fusion_config::EXACT_MATCH_BOOST, fusion_config::HEADER_MATCH_BOOST)
    }
}

impl Default for FusionStrategy {
    /// Default to VocabularyOptimized for TTRPG RAG applications.
    /// Uses BM25_WEIGHT (0.4) and VECTOR_WEIGHT (0.6) from MDMAI config.
    fn default() -> Self {
        FusionStrategy::VocabularyOptimized
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_doc(id: &str, content: &str) -> SearchDocument {
        SearchDocument {
            id: id.to_string(),
            content: content.to_string(),
            source: "test".to_string(),
            source_type: "test".to_string(),
            page_number: None,
            chunk_index: None,
            campaign_id: None,
            session_id: None,
            created_at: "2024-01-01".to_string(),
            metadata: HashMap::new(),
            ..Default::default()
        }
    }

    #[test]
    fn test_rrf_formula_rank_0() {
        let engine = RRFEngine::with_k(60);
        let score = engine.score_at_rank(0, 1.0);
        // 1.0 / (60 + 0 + 1) = 1/61 ~ 0.01639
        assert!((score - 0.01639).abs() < 0.001);
    }

    #[test]
    fn test_rrf_formula_rank_1() {
        let engine = RRFEngine::with_k(60);
        let score = engine.score_at_rank(1, 1.0);
        // 1.0 / (60 + 1 + 1) = 1/62 ~ 0.01613
        assert!((score - 0.01613).abs() < 0.001);
    }

    #[test]
    fn test_max_theoretical_score() {
        let engine = RRFEngine::with_k(60);
        let max_score = engine.max_theoretical_score(&[0.5, 0.5]);
        // 0.5/61 + 0.5/61 = 1/61 ~ 0.01639
        assert!((max_score - 0.01639).abs() < 0.001);
    }

    #[test]
    fn test_fuse_keyword_semantic() {
        // Use lenient config to disable min_score filtering for test
        let engine = RRFEngine::new(RRFConfig {
            k: 60,
            normalize_scores: false,
            min_score: 0.0,  // Disable filtering for test
            ..Default::default()
        });

        let keyword = vec![
            (make_doc("doc1", "first"), 1.0, "test".to_string()),
            (make_doc("doc2", "second"), 0.9, "test".to_string()),
            (make_doc("doc3", "third"), 0.8, "test".to_string()),
        ];

        let semantic = vec![
            (make_doc("doc2", "second"), 1.0, "test".to_string()), // doc2 is #1 in semantic
            (make_doc("doc1", "first"), 0.9, "test".to_string()),  // doc1 is #2 in semantic
            (make_doc("doc4", "fourth"), 0.8, "test".to_string()), // doc4 only in semantic
        ];

        let results = engine.fuse_keyword_semantic(keyword, semantic, 0.5, 0.5);

        // doc1: rank 0 keyword + rank 1 semantic
        // doc2: rank 1 keyword + rank 0 semantic
        // Both should have similar scores but doc2 might edge out slightly
        assert!(results.len() >= 3);

        // doc1 and doc2 should be in top results with overlap_count = 2
        let doc1 = results.iter().find(|r| r.document.id == "doc1").unwrap();
        let doc2 = results.iter().find(|r| r.document.id == "doc2").unwrap();

        assert_eq!(doc1.overlap_count, 2);
        assert_eq!(doc2.overlap_count, 2);
        assert!(doc1.keyword_rank.is_some());
        assert!(doc1.semantic_rank.is_some());
    }

    #[test]
    fn test_overlap_boosting() {
        // Use lenient config to disable min_score filtering for test
        let engine = RRFEngine::new(RRFConfig {
            k: 60,
            normalize_scores: false,
            min_score: 0.0,  // Disable filtering for test
            ..Default::default()
        });

        // doc1 appears in both, doc2 only in keyword
        let keyword = vec![
            (make_doc("doc1", "both"), 1.0, "test".to_string()),
            (make_doc("doc2", "keyword only"), 0.9, "test".to_string()),
        ];

        let semantic = vec![(make_doc("doc1", "both"), 1.0, "test".to_string())];

        let results = engine.fuse_keyword_semantic(keyword, semantic, 0.5, 0.5);

        // doc1 should have higher score due to appearing in both
        assert_eq!(results[0].document.id, "doc1");
        assert_eq!(results[0].overlap_count, 2);
    }

    #[test]
    fn test_fusion_strategy_weights() {
        assert_eq!(FusionStrategy::Balanced.weights(), (0.5, 0.5));
        assert_eq!(FusionStrategy::KeywordHeavy.weights(), (0.7, 0.3));
        assert_eq!(FusionStrategy::SemanticHeavy.weights(), (0.3, 0.7));
    }

    #[test]
    fn test_fusion_strategy_vocabulary_optimized() {
        let (kw, sem) = FusionStrategy::VocabularyOptimized.weights();
        // Should use fusion_config values: BM25=0.4, Vector=0.6
        assert_eq!(kw, fusion_config::BM25_WEIGHT);
        assert_eq!(sem, fusion_config::VECTOR_WEIGHT);
        assert!((kw + sem - 1.0).abs() < 0.01, "Weights should sum to 1.0");
    }

    #[test]
    fn test_fusion_strategy_default_is_vocabulary_optimized() {
        let default_strategy = FusionStrategy::default();
        let vocab_strategy = FusionStrategy::VocabularyOptimized;
        assert_eq!(default_strategy.weights(), vocab_strategy.weights());
    }

    #[test]
    fn test_rrf_config_default_uses_vocabulary() {
        let config = RRFConfig::default();
        assert_eq!(config.k, fusion_config::RRF_K as u32);
        assert_eq!(config.min_score, fusion_config::MIN_SCORE);
        assert_eq!(config.max_results, fusion_config::MAX_RESULTS);
    }

    #[test]
    fn test_rrf_config_presets() {
        let lenient = RRFConfig::lenient();
        let strict = RRFConfig::strict();

        // Lenient should have no min_score and more results
        assert_eq!(lenient.min_score, 0.0);
        assert!(lenient.max_results > strict.max_results);

        // Strict should have higher min_score and fewer results
        assert!(strict.min_score > fusion_config::MIN_SCORE);
        assert!(strict.max_results < fusion_config::MAX_RESULTS);
    }

    #[test]
    fn test_boost_factors() {
        let (exact, header) = FusionStrategy::boost_factors();
        assert_eq!(exact, fusion_config::EXACT_MATCH_BOOST);
        assert_eq!(header, fusion_config::HEADER_MATCH_BOOST);
        assert!(exact > 1.0, "Exact match boost should be > 1.0");
        assert!(header > 1.0, "Header match boost should be > 1.0");
    }

    #[test]
    fn test_normalized_scores() {
        // Use min_score: 0.0 to prevent filtering
        let engine = RRFEngine::new(RRFConfig {
            k: 60,
            normalize_scores: true,
            min_score: 0.0,  // Disable filtering for test
            ..Default::default()
        });

        let keyword = vec![
            (make_doc("doc1", "first"), 1.0, "test".to_string()),
            (make_doc("doc2", "second"), 0.9, "test".to_string()),
        ];

        let semantic = vec![(make_doc("doc1", "first"), 1.0, "test".to_string())];

        let results = engine.fuse_keyword_semantic(keyword, semantic, 0.5, 0.5);

        // Top result should have normalized score of 1.0
        assert!((results[0].score - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_generic_fusion() {
        // Use lenient config to prevent min_score filtering
        let engine = RRFEngine::new(RRFConfig::lenient());

        let set1 = vec![
            RankedItem {
                item: "apple",
                rank: 0,
                original_score: Some(1.0),
                source: "set1".into(),
            },
            RankedItem {
                item: "banana",
                rank: 1,
                original_score: Some(0.9),
                source: "set1".into(),
            },
        ];

        let set2 = vec![
            RankedItem {
                item: "banana",
                rank: 0,
                original_score: Some(1.0),
                source: "set2".into(),
            },
            RankedItem {
                item: "apple",
                rank: 1,
                original_score: Some(0.9),
                source: "set2".into(),
            },
        ];

        let result_sets = vec![(0.5, set1), (0.5, set2)];

        let results = engine.fuse(result_sets, |s| *s);

        // Both items appear in both sets with opposite ranks
        // Scores should be identical
        assert_eq!(results.len(), 2);
        let apple = results.iter().find(|r| &*r.item == "apple").unwrap();
        let banana = results.iter().find(|r| &*r.item == "banana").unwrap();
        assert!((apple.score - banana.score).abs() < 0.001);
    }
}
