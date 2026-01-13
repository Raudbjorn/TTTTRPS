//! Result Ranker Module
//!
//! Combines dense and sparse search results using Reciprocal Rank Fusion (RRF).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{AntonymMapper, QueryConstraints};

// ============================================================================
// Types
// ============================================================================

/// Score breakdown for transparency
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    /// Semantic/dense search score
    pub semantic_score: f32,
    /// Keyword/sparse search score
    pub keyword_score: f32,
    /// Bonus for matching required attributes
    pub attribute_match_bonus: f32,
    /// Penalty for antonym presence
    pub antonym_penalty: f32,
    /// Boost for exact entity matches
    pub exact_match_boost: f32,
    /// Final combined score
    pub final_score: f32,
}

/// Ranking configuration
#[derive(Debug, Clone)]
pub struct RankingConfig {
    /// RRF constant k (typically 60)
    pub rrf_k: f32,
    /// Weight for semantic/dense results
    pub semantic_weight: f32,
    /// Weight for keyword/sparse results
    pub keyword_weight: f32,
    /// Bonus multiplier for attribute matches
    pub attribute_match_bonus: f32,
    /// Boost multiplier for exact entity matches
    pub exact_match_boost: f32,
    /// Whether to hard exclude results with excluded attributes
    pub hard_exclude_veto: bool,
}

impl Default for RankingConfig {
    fn default() -> Self {
        Self {
            rrf_k: 60.0,
            semantic_weight: 0.6,
            keyword_weight: 0.4,
            attribute_match_bonus: 0.2,
            exact_match_boost: 0.3,
            hard_exclude_veto: true,
        }
    }
}

/// A search candidate with ID and score
#[derive(Debug, Clone)]
pub struct SearchCandidate {
    /// Document ID
    pub id: String,
    /// Raw search score
    pub score: f32,
    /// Document content for exact matching
    pub content: String,
}

/// A ranked result with full breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedResult {
    /// Document ID
    pub id: String,
    /// Score breakdown
    pub breakdown: ScoreBreakdown,
    /// Whether this result was vetoed
    pub vetoed: bool,
}

// ============================================================================
// Result Ranker
// ============================================================================

/// Ranks and fuses search results using RRF
pub struct ResultRanker {
    config: RankingConfig,
    antonym_mapper: AntonymMapper,
}

impl Default for ResultRanker {
    fn default() -> Self {
        Self::new()
    }
}

impl ResultRanker {
    /// Create a new result ranker with default config
    pub fn new() -> Self {
        Self::with_config(RankingConfig::default())
    }

    /// Create a result ranker with custom config
    pub fn with_config(config: RankingConfig) -> Self {
        Self {
            config,
            antonym_mapper: AntonymMapper::new(),
        }
    }

    /// Set the antonym mapper
    pub fn with_antonym_mapper(mut self, mapper: AntonymMapper) -> Self {
        self.antonym_mapper = mapper;
        self
    }

    /// Fuse dense and sparse results using Reciprocal Rank Fusion
    ///
    /// # Returns
    /// Map of document ID to (semantic_rrf_score, keyword_rrf_score)
    pub fn fuse_rrf(
        &self,
        dense_results: &[SearchCandidate],
        sparse_results: &[SearchCandidate],
    ) -> HashMap<String, (f32, f32)> {
        let mut scores: HashMap<String, (f32, f32)> = HashMap::new();
        let k = self.config.rrf_k;

        // Calculate RRF scores for dense results
        for (rank, candidate) in dense_results.iter().enumerate() {
            let rrf_score = 1.0 / (k + rank as f32 + 1.0);
            scores.entry(candidate.id.clone())
                .or_insert((0.0, 0.0))
                .0 = rrf_score;
        }

        // Calculate RRF scores for sparse results
        for (rank, candidate) in sparse_results.iter().enumerate() {
            let rrf_score = 1.0 / (k + rank as f32 + 1.0);
            scores.entry(candidate.id.clone())
                .or_insert((0.0, 0.0))
                .1 = rrf_score;
        }

        scores
    }

    /// Full ranking pipeline with score breakdown
    pub fn rank(
        &self,
        dense_results: &[SearchCandidate],
        sparse_results: &[SearchCandidate],
        constraints: &QueryConstraints,
        doc_attributes: &HashMap<String, Vec<String>>,
    ) -> Vec<RankedResult> {
        let rrf_scores = self.fuse_rrf(dense_results, sparse_results);

        // Build content map for exact matching
        let content_map: HashMap<String, String> = dense_results.iter()
            .chain(sparse_results.iter())
            .map(|c| (c.id.clone(), c.content.clone()))
            .collect();

        let mut results: Vec<RankedResult> = rrf_scores
            .into_iter()
            .map(|(id, (semantic, keyword))| {
                let attrs = doc_attributes.get(&id)
                    .map(|v| v.as_slice())
                    .unwrap_or(&[]);

                let breakdown = self.calculate_breakdown(
                    semantic,
                    keyword,
                    constraints,
                    attrs,
                    content_map.get(&id).map(|s| s.as_str()),
                );

                // Check for hard veto
                let vetoed = self.config.hard_exclude_veto
                    && self.antonym_mapper.has_excluded(
                        &constraints.excluded_attributes,
                        &attrs.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                    );

                RankedResult {
                    id,
                    breakdown,
                    vetoed,
                }
            })
            .collect();

        // Sort by final score descending, vetoed items last
        results.sort_by(|a, b| {
            if a.vetoed != b.vetoed {
                return a.vetoed.cmp(&b.vetoed);
            }
            b.breakdown.final_score
                .partial_cmp(&a.breakdown.final_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }

    /// Calculate full score breakdown
    fn calculate_breakdown(
        &self,
        semantic_rrf: f32,
        keyword_rrf: f32,
        constraints: &QueryConstraints,
        doc_attrs: &[String],
        content: Option<&str>,
    ) -> ScoreBreakdown {
        // Base scores with weights
        let semantic_score = semantic_rrf * self.config.semantic_weight;
        let keyword_score = keyword_rrf * self.config.keyword_weight;

        // Attribute match bonus
        let query_attrs: Vec<String> = constraints.required_attributes
            .iter()
            .map(|a| a.value.clone())
            .collect();

        let matching_count = query_attrs.iter()
            .filter(|qa| doc_attrs.iter().any(|da| da.to_lowercase() == qa.to_lowercase()))
            .count();

        // Calculate attribute match ratio, defaulting to 0 if no query attributes
        let attribute_match_bonus = match query_attrs.len() {
            0 => 0.0,
            n => (matching_count as f32 / n as f32) * self.config.attribute_match_bonus,
        };

        // Antonym penalty
        let antonym_penalty = self.antonym_mapper.calculate_penalty(&query_attrs, doc_attrs);

        // Exact match boost
        let exact_match_boost = if let Some(text) = content {
            let text_lower = text.to_lowercase();
            let exact_matches = constraints.exact_match_entities
                .iter()
                .filter(|e| text_lower.contains(&e.to_lowercase()))
                .count();

            if !constraints.exact_match_entities.is_empty() {
                (exact_matches as f32 / constraints.exact_match_entities.len() as f32)
                    * self.config.exact_match_boost
            } else {
                0.0
            }
        } else {
            0.0
        };

        // Calculate final score
        let final_score = (semantic_score + keyword_score + attribute_match_bonus + exact_match_boost)
            * antonym_penalty;

        ScoreBreakdown {
            semantic_score,
            keyword_score,
            attribute_match_bonus,
            antonym_penalty,
            exact_match_boost,
            final_score,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_candidate(id: &str, score: f32, content: &str) -> SearchCandidate {
        SearchCandidate {
            id: id.to_string(),
            score,
            content: content.to_string(),
        }
    }

    #[test]
    fn test_rrf_fusion() {
        let ranker = ResultRanker::new();

        let dense = vec![
            make_candidate("doc1", 0.9, ""),
            make_candidate("doc2", 0.8, ""),
        ];
        let sparse = vec![
            make_candidate("doc2", 0.95, ""),
            make_candidate("doc3", 0.85, ""),
        ];

        let fused = ranker.fuse_rrf(&dense, &sparse);

        // doc1 should only have semantic score
        assert!(fused["doc1"].0 > 0.0);
        assert_eq!(fused["doc1"].1, 0.0);

        // doc2 should have both
        assert!(fused["doc2"].0 > 0.0);
        assert!(fused["doc2"].1 > 0.0);

        // doc3 should only have keyword score
        assert_eq!(fused["doc3"].0, 0.0);
        assert!(fused["doc3"].1 > 0.0);
    }

    #[test]
    fn test_rank_with_constraints() {
        let ranker = ResultRanker::new();

        let dense = vec![
            make_candidate("fire_doc", 0.9, "Fire damage spell"),
            make_candidate("cold_doc", 0.85, "Cold damage spell"),
        ];
        let sparse = vec![
            make_candidate("fire_doc", 0.9, "Fire damage spell"),
        ];

        let constraints = QueryConstraints {
            required_attributes: vec![
                super::super::RequiredAttribute {
                    category: "damage_type".to_string(),
                    value: "fire".to_string(),
                    required: true,
                }
            ],
            ..Default::default()
        };

        let mut doc_attrs = HashMap::new();
        doc_attrs.insert("fire_doc".to_string(), vec!["fire".to_string()]);
        doc_attrs.insert("cold_doc".to_string(), vec!["cold".to_string()]);

        let results = ranker.rank(&dense, &sparse, &constraints, &doc_attrs);

        // Fire doc should rank higher due to attribute match bonus
        assert_eq!(results[0].id, "fire_doc");
        assert!(results[0].breakdown.attribute_match_bonus > 0.0);
    }

    #[test]
    fn test_hard_veto() {
        let ranker = ResultRanker::new();

        let dense = vec![
            make_candidate("undead_doc", 0.9, "Undead creature"),
        ];

        let constraints = QueryConstraints {
            excluded_attributes: vec!["undead".to_string()],
            ..Default::default()
        };

        let mut doc_attrs = HashMap::new();
        doc_attrs.insert("undead_doc".to_string(), vec!["undead".to_string()]);

        let results = ranker.rank(&dense, &[], &constraints, &doc_attrs);

        assert!(results[0].vetoed);
    }

    #[test]
    fn test_exact_match_boost() {
        let ranker = ResultRanker::new();

        let dense = vec![
            make_candidate("doc1", 0.9, "The Goblin King attacks"),
            make_candidate("doc2", 0.9, "A regular goblin"),
        ];

        let constraints = QueryConstraints {
            exact_match_entities: vec!["Goblin King".to_string()],
            ..Default::default()
        };

        let results = ranker.rank(&dense, &[], &constraints, &HashMap::new());

        // doc1 has exact match, should have boost
        assert!(results[0].breakdown.exact_match_boost > 0.0);
    }
}
