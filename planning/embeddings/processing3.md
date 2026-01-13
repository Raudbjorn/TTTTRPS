Great question! 🎯 This is a classic and well-known limitation of pure semantic search — embeddings capture *semantic similarity* but often fumble on **attribute specificity** (colors, sizes, negations, quantities).

Let me break this down across the three phases you mentioned:

## The Core Problem 🧠

Embeddings see: `"BLACK sand beach"` ≈ `"WHITE sand beach"` (both are sand beaches!)
User means: `"BLACK sand beach"` ≠ `"WHITE sand beach"` (completely different vibe!)

## Solutions Across the Pipeline

### 1. **Pre-Processing: Critical Attribute Extraction**

Extract "hard constraint" terms that MUST match:

```rust
// src/preprocessing/critical_terms.rs

use std::collections::HashSet;

/// Categories of terms that often require exact/near-exact matching
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CriticalTermCategory {
    Color,
    Size,
    Quantity,
    Negation,
    Material,
    TimeOfDay,
    Weather,
    Season,
}

#[derive(Debug, Clone)]
pub struct CriticalTerm {
    pub term: String,
    pub category: CriticalTermCategory,
    pub position: usize,
    /// Antonyms that should be penalized heavily
    pub antonyms: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ExtractedQuery {
    pub original: String,
    pub normalized: String,
    pub critical_terms: Vec<CriticalTerm>,
    /// Query with critical terms removed (for pure semantic search)
    pub semantic_query: String,
}

/// Builds a critical term extractor with configurable vocabularies
pub struct CriticalTermExtractor {
    colors: HashSet<String>,
    color_antonyms: std::collections::HashMap<String, Vec<String>>,
    sizes: HashSet<String>,
    negations: HashSet<String>,
    time_of_day: HashSet<String>,
    weather: HashSet<String>,
}

impl Default for CriticalTermExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl CriticalTermExtractor {
    pub fn new() -> Self {
        let mut color_antonyms = std::collections::HashMap::new();
        color_antonyms.insert("black".into(), vec!["white".into(), "light".into()]);
        color_antonyms.insert("white".into(), vec!["black".into(), "dark".into()]);
        color_antonyms.insert("dark".into(), vec!["light".into(), "bright".into()]);
        color_antonyms.insert("light".into(), vec!["dark".into()]);
        color_antonyms.insert("red".into(), vec!["green".into(), "blue".into()]);
        color_antonyms.insert("blue".into(), vec!["orange".into(), "red".into()]);
        // ... extend as needed

        Self {
            colors: [
                "black", "white", "red", "blue", "green", "yellow", "orange",
                "purple", "pink", "brown", "gray", "grey", "golden", "silver",
                "dark", "light", "bright", "vibrant", "muted", "pastel",
            ].into_iter().map(String::from).collect(),
            
            color_antonyms,
            
            sizes: [
                "tiny", "small", "medium", "large", "huge", "massive",
                "narrow", "wide", "tall", "short", "deep", "shallow",
            ].into_iter().map(String::from).collect(),
            
            negations: [
                "no", "not", "without", "except", "excluding", "never",
                "none", "empty", "absent", "missing", "lack", "lacking",
            ].into_iter().map(String::from).collect(),
            
            time_of_day: [
                "sunrise", "sunset", "dawn", "dusk", "morning", "afternoon",
                "evening", "night", "midnight", "noon", "twilight", "golden hour",
            ].into_iter().map(String::from).collect(),
            
            weather: [
                "sunny", "cloudy", "rainy", "stormy", "foggy", "misty",
                "snowy", "clear", "overcast", "hazy",
            ].into_iter().map(String::from).collect(),
        }
    }

    /// Extract critical terms from a query
    pub fn extract(&self, query: &str) -> ExtractedQuery {
        let normalized = query.to_lowercase();
        let words: Vec<&str> = normalized.split_whitespace().collect();
        let mut critical_terms = Vec::new();
        let mut critical_positions = HashSet::new();

        for (pos, word) in words.iter().enumerate() {
            let clean_word = word.trim_matches(|c: char| !c.is_alphanumeric());
            
            if let Some(term) = self.classify_term(clean_word, pos) {
                critical_positions.insert(pos);
                critical_terms.push(term);
            }
        }

        // Build semantic query (critical terms removed for pure embedding search)
        let semantic_query: String = words
            .iter()
            .enumerate()
            .filter(|(pos, _)| !critical_positions.contains(pos))
            .map(|(_, word)| *word)
            .collect::<Vec<_>>()
            .join(" ");

        ExtractedQuery {
            original: query.to_string(),
            normalized,
            critical_terms,
            semantic_query,
        }
    }

    fn classify_term(&self, word: &str, position: usize) -> Option<CriticalTerm> {
        if self.colors.contains(word) {
            return Some(CriticalTerm {
                term: word.to_string(),
                category: CriticalTermCategory::Color,
                position,
                antonyms: self.color_antonyms
                    .get(word)
                    .cloned()
                    .unwrap_or_default(),
            });
        }

        if self.sizes.contains(word) {
            return Some(CriticalTerm {
                term: word.to_string(),
                category: CriticalTermCategory::Size,
                position,
                antonyms: vec![],
            });
        }

        if self.negations.contains(word) {
            return Some(CriticalTerm {
                term: word.to_string(),
                category: CriticalTermCategory::Negation,
                position,
                antonyms: vec![],
            });
        }

        if self.time_of_day.contains(word) {
            return Some(CriticalTerm {
                term: word.to_string(),
                category: CriticalTermCategory::TimeOfDay,
                position,
                antonyms: vec![],
            });
        }

        if self.weather.contains(word) {
            return Some(CriticalTerm {
                term: word.to_string(),
                category: CriticalTermCategory::Weather,
                position,
                antonyms: vec![],
            });
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_color_from_beach_query() {
        let extractor = CriticalTermExtractor::new();
        let result = extractor.extract("BLACK sand beach");

        assert_eq!(result.critical_terms.len(), 1);
        assert_eq!(result.critical_terms[0].term, "black");
        assert_eq!(result.critical_terms[0].category, CriticalTermCategory::Color);
        assert!(result.critical_terms[0].antonyms.contains(&"white".to_string()));
        assert_eq!(result.semantic_query, "sand beach");
    }

    #[test]
    fn handles_multiple_critical_terms() {
        let extractor = CriticalTermExtractor::new();
        let result = extractor.extract("dark stormy beach at sunset");

        assert_eq!(result.critical_terms.len(), 3); // dark, stormy, sunset
    }
}
```

---

### 2. **Hybrid Search: Combine Semantic + Keyword**

This is the **most effective** approach for your use case:

```rust
// src/search/hybrid.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a document with both embedding and keyword metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedDocument {
    pub id: String,
    pub text: String,
    pub embedding: Vec<f32>,
    /// Pre-extracted attributes for fast filtering
    pub attributes: DocumentAttributes,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocumentAttributes {
    pub colors: Vec<String>,
    pub sizes: Vec<String>,
    pub time_of_day: Option<String>,
    pub weather: Option<String>,
    pub negated_terms: Vec<String>,
}

/// Search result with component scores for transparency
#[derive(Debug, Clone)]
pub struct HybridSearchResult {
    pub document_id: String,
    pub semantic_score: f32,
    pub keyword_score: f32,
    pub attribute_match_score: f32,
    pub antonym_penalty: f32,
    pub final_score: f32,
}

/// Configuration for hybrid search scoring
#[derive(Debug, Clone)]
pub struct HybridSearchConfig {
    /// Weight for semantic similarity (0.0 - 1.0)
    pub semantic_weight: f32,
    /// Weight for keyword/BM25 score (0.0 - 1.0)  
    pub keyword_weight: f32,
    /// Bonus for matching critical attributes
    pub attribute_match_bonus: f32,
    /// Penalty multiplier for antonym presence (0.0 - 1.0, lower = harsher)
    pub antonym_penalty_factor: f32,
    /// If true, antonym presence completely disqualifies result
    pub hard_antonym_filter: bool,
}

impl Default for HybridSearchConfig {
    fn default() -> Self {
        Self {
            semantic_weight: 0.6,
            keyword_weight: 0.4,
            attribute_match_bonus: 0.15,
            antonym_penalty_factor: 0.1, // Harsh penalty: score * 0.1
            hard_antonym_filter: false,
        }
    }
}

pub struct HybridSearcher {
    config: HybridSearchConfig,
}

impl HybridSearcher {
    pub fn new(config: HybridSearchConfig) -> Self {
        Self { config }
    }

    /// Score a single document against an extracted query
    pub fn score_document(
        &self,
        query: &super::super::preprocessing::critical_terms::ExtractedQuery,
        doc: &IndexedDocument,
        semantic_score: f32,
        keyword_score: f32,
    ) -> Option<HybridSearchResult> {
        // Check for antonym presence
        let antonym_penalty = self.calculate_antonym_penalty(query, &doc.attributes);
        
        if self.config.hard_antonym_filter && antonym_penalty < 1.0 {
            // Document contains an antonym of a critical term - filter out
            log::debug!(
                "Document {} filtered: contains antonym of query critical term",
                doc.id
            );
            return None;
        }

        // Calculate attribute match bonus
        let attribute_match_score = self.calculate_attribute_match(query, &doc.attributes);

        // Combine scores
        let base_score = (self.config.semantic_weight * semantic_score)
            + (self.config.keyword_weight * keyword_score);
        
        let boosted_score = base_score + (self.config.attribute_match_bonus * attribute_match_score);
        let final_score = boosted_score * antonym_penalty;

        Some(HybridSearchResult {
            document_id: doc.id.clone(),
            semantic_score,
            keyword_score,
            attribute_match_score,
            antonym_penalty,
            final_score,
        })
    }

    fn calculate_antonym_penalty(
        &self,
        query: &super::super::preprocessing::critical_terms::ExtractedQuery,
        doc_attrs: &DocumentAttributes,
    ) -> f32 {
        for critical_term in &query.critical_terms {
            for antonym in &critical_term.antonyms {
                // Check if document contains the antonym
                let doc_text_lower: Vec<&str> = doc_attrs.colors.iter()
                    .map(|s| s.as_str())
                    .collect();
                
                if doc_text_lower.contains(&antonym.as_str()) {
                    log::info!(
                        "Antonym detected: query has '{}', doc has '{}'",
                        critical_term.term,
                        antonym
                    );
                    return self.config.antonym_penalty_factor;
                }
            }
        }
        1.0 // No penalty
    }

    fn calculate_attribute_match(
        &self,
        query: &super::super::preprocessing::critical_terms::ExtractedQuery,
        doc_attrs: &DocumentAttributes,
    ) -> f32 {
        if query.critical_terms.is_empty() {
            return 0.0;
        }

        let mut matches = 0;
        for critical_term in &query.critical_terms {
            match critical_term.category {
                super::super::preprocessing::critical_terms::CriticalTermCategory::Color => {
                    if doc_attrs.colors.iter().any(|c| c.to_lowercase() == critical_term.term) {
                        matches += 1;
                    }
                }
                // Add other category matches...
                _ => {}
            }
        }

        matches as f32 / query.critical_terms.len() as f32
    }

    /// Rank a batch of results
    pub fn rank_results(&self, mut results: Vec<HybridSearchResult>) -> Vec<HybridSearchResult> {
        results.sort_by(|a, b| {
            b.final_score
                .partial_cmp(&a.final_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results
    }
}
```

---

### 3. **Post-Processing: Re-ranker with Attribute Verification**

For when you need a second pass:

```rust
// src/postprocessing/reranker.rs

use std::collections::HashSet;

/// A re-ranker that verifies critical attribute presence
pub struct AttributeVerifyingReranker {
    /// Minimum attribute match ratio to keep result (0.0 - 1.0)
    min_attribute_match_ratio: f32,
    /// Whether to boost results with exact critical term matches
    boost_exact_matches: bool,
    exact_match_boost: f32,
}

impl AttributeVerifyingReranker {
    pub fn new(min_attribute_match_ratio: f32) -> Self {
        Self {
            min_attribute_match_ratio,
            boost_exact_matches: true,
            exact_match_boost: 0.2,
        }
    }

    /// Re-rank results, filtering those that don't meet attribute requirements
    pub fn rerank<'a>(
        &self,
        query_critical_terms: &[String],
        results: impl Iterator<Item = (&'a str, &'a str, f32)>, // (id, text, score)
    ) -> Vec<RerankedResult> {
        let query_terms: HashSet<&str> = query_critical_terms
            .iter()
            .map(|s| s.as_str())
            .collect();

        let mut reranked: Vec<RerankedResult> = results
            .filter_map(|(id, text, original_score)| {
                let text_lower = text.to_lowercase();
                let text_words: HashSet<&str> = text_lower.split_whitespace().collect();

                // Count how many critical terms are present
                let matches: usize = query_terms
                    .iter()
                    .filter(|term| text_words.contains(*term))
                    .count();

                let match_ratio = if query_terms.is_empty() {
                    1.0
                } else {
                    matches as f32 / query_terms.len() as f32
                };

                // Filter if below threshold
                if match_ratio < self.min_attribute_match_ratio {
                    log::info!(
                        "Filtered result {}: attribute match ratio {} < threshold {}",
                        id,
                        match_ratio,
                        self.min_attribute_match_ratio
                    );
                    return None;
                }

                // Calculate new score
                let boost = if self.boost_exact_matches && match_ratio == 1.0 {
                    self.exact_match_boost
                } else {
                    0.0
                };

                let new_score = original_score * (1.0 + boost) * (0.5 + 0.5 * match_ratio);

                Some(RerankedResult {
                    id: id.to_string(),
                    original_score,
                    reranked_score: new_score,
                    attribute_match_ratio: match_ratio,
                })
            })
            .collect();

        reranked.sort_by(|a, b| {
            b.reranked_score
                .partial_cmp(&a.reranked_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        log::info!(
            "Re-ranking complete: {} results after filtering",
            reranked.len()
        );

        reranked
    }
}

#[derive(Debug, Clone)]
pub struct RerankedResult {
    pub id: String,
    pub original_score: f32,
    pub reranked_score: f32,
    pub attribute_match_ratio: f32,
}
```

---

### 4. **On Model Tuning: Should You Fine-tune?** 🤔

Short answer: **Probably not as your first approach.**

| Approach | Effort | Effectiveness for Your Case |
|----------|--------|----------------------------|
| Hybrid search (semantic + keyword) | Low | ⭐⭐⭐⭐⭐ |
| Pre-extracted attributes + filtering | Low | ⭐⭐⭐⭐ |
| Post-processing re-ranker | Low | ⭐⭐⭐⭐ |
| Fine-tuning embeddings | High | ⭐⭐⭐ (diminishing returns) |
| ColBERT/multi-vector | Medium | ⭐⭐⭐⭐ |

**If you DO want to fine-tune**, here's the approach:

```rust
// src/training/contrastive_pairs.rs

use serde::{Deserialize, Serialize};

/// Training pair for contrastive learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContrastivePair {
    pub anchor: String,
    pub positive: String,    // Should be similar
    pub hard_negative: String, // Semantically similar but attribute-different
}

/// Generate training pairs that teach the model attribute sensitivity
pub fn generate_attribute_contrastive_pairs() -> Vec<ContrastivePair> {
    vec![
        // Color contrasts
        ContrastivePair {
            anchor: "black sand beach volcanic".into(),
            positive: "dark volcanic beach with black sand".into(),
            hard_negative: "white sand beach tropical".into(), // Same "beach" but wrong color!
        },
        ContrastivePair {
            anchor: "golden sunset over ocean".into(),
            positive: "orange gold sunset sea horizon".into(),
            hard_negative: "blue hour ocean twilight".into(),
        },
        ContrastivePair {
            anchor: "snowy mountain peak winter".into(),
            positive: "snow covered mountain summit".into(),
            hard_negative: "green mountain summer meadow".into(),
        },
        // Time of day contrasts
        ContrastivePair {
            anchor: "city skyline at night".into(),
            positive: "urban nighttime cityscape lights".into(),
            hard_negative: "city skyline daytime bright".into(),
        },
        // Add more pairs based on your domain (film/TV locations for Massif!)
    ]
}

/// For training with a library like `candle` or calling out to Python
#[derive(Debug, Serialize)]
pub struct TrainingBatch {
    pub anchors: Vec<String>,
    pub positives: Vec<String>,
    pub hard_negatives: Vec<String>,
}

impl TrainingBatch {
    pub fn from_pairs(pairs: Vec<ContrastivePair>) -> Self {
        Self {
            anchors: pairs.iter().map(|p| p.anchor.clone()).collect(),
            positives: pairs.iter().map(|p| p.positive.clone()).collect(),
            hard_negatives: pairs.iter().map(|p| p.hard_negative.clone()).collect(),
        }
    }
}
```

---

### 5. **Putting It Together: A Complete Search Pipeline**

```rust
// src/lib.rs

pub mod preprocessing {
    pub mod critical_terms;
}
pub mod search {
    pub mod hybrid;
}
pub mod postprocessing {
    pub mod reranker;
}

// src/pipeline.rs

use crate::preprocessing::critical_terms::{CriticalTermExtractor, ExtractedQuery};
use crate::postprocessing::reranker::{AttributeVerifyingReranker, RerankedResult};

pub struct SemanticSearchPipeline {
    term_extractor: CriticalTermExtractor,
    reranker: AttributeVerifyingReranker,
}

impl SemanticSearchPipeline {
    pub fn new() -> Self {
        Self {
            term_extractor: CriticalTermExtractor::new(),
            reranker: AttributeVerifyingReranker::new(0.5), // At least 50% of critical terms must match
        }
    }

    /// Full search pipeline
    pub fn search(
        &self,
        query: &str,
        // In practice, these would come from your vector DB + keyword index
        get_semantic_results: impl Fn(&str) -> Vec<(String, String, f32)>,
        get_keyword_results: impl Fn(&str) -> Vec<(String, String, f32)>,
    ) -> Result<Vec<RerankedResult>, SearchError> {
        // 1. Pre-process: Extract critical terms
        let extracted = self.term_extractor.extract(query);
        log::info!(
            "Extracted {} critical terms from query",
            extracted.critical_terms.len()
        );

        // 2. Run both searches
        let semantic_results = get_semantic_results(&extracted.semantic_query);
        let keyword_results = get_keyword_results(&extracted.original);

        // 3. Merge results (simplified - you'd want proper fusion here)
        let merged = self.merge_results(semantic_results, keyword_results);

        // 4. Post-process: Re-rank with attribute verification
        let critical_term_strings: Vec<String> = extracted
            .critical_terms
            .iter()
            .map(|t| t.term.clone())
            .collect();

        let reranked = self.reranker.rerank(
            &critical_term_strings,
            merged.iter().map(|(id, text, score)| (id.as_str(), text.as_str(), *score)),
        );

        log::info!(
            "Search complete: {} results after re-ranking",
            reranked.len()
        );

        Ok(reranked)
    }

    fn merge_results(
        &self,
        semantic: Vec<(String, String, f32)>,
        keyword: Vec<(String, String, f32)>,
    ) -> Vec<(String, String, f32)> {
        use std::collections::HashMap;
        
        let mut merged: HashMap<String, (String, f32, f32)> = HashMap::new();

        for (id, text, score) in semantic {
            merged.insert(id, (text, score, 0.0));
        }

        for (id, text, score) in keyword {
            merged
                .entry(id)
                .and_modify(|(_, _, kw_score)| *kw_score = score)
                .or_insert((text, 0.0, score));
        }

        // Reciprocal Rank Fusion (simplified)
        merged
            .into_iter()
            .map(|(id, (text, sem_score, kw_score))| {
                let combined = 0.6 * sem_score + 0.4 * kw_score;
                (id, text, combined)
            })
            .collect()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("Search backend unavailable: {0}")]
    BackendUnavailable(String),
    #[error("Invalid query: {0}")]
    InvalidQuery(String),
}

impl Default for SemanticSearchPipeline {
    fn default() -> Self {
        Self::new()
    }
}
```

---

## TL;DR Recommendations 🎯

1. **Start with hybrid search** — combine your vector search with a keyword index (Tantivy, Meilisearch, or even PostgreSQL full-text search). This alone solves 80% of the problem.

2. **Extract critical terms at query time** — identify colors, sizes, negations, etc., and use them for hard filtering or heavy boosting.

3. **Pre-index attributes on documents** — when you generate embeddings, also extract and store `{colors: ["black"], weather: ["sunny"], ...}` as structured metadata.

4. **Post-process with antonym penalties** — if user searches "black" and doc contains "white", that's a strong signal to demote.

5. **Fine-tuning is last resort** — it's expensive, needs lots of contrastive pairs, and hybrid search usually gets you there faster.

Want me to expand on any of these? I can also show integration with specific vector DBs (Qdrant, Milvus, pgvector) or keyword engines (Tantivy) if that would help! 🚀

Let me search for real-world implementations of this! 🔍Excellent question! Here's a curated list of repos and resources that tackle various aspects of this problem. 🗂️

---

## 📚 **Comprehensive Repo List by Approach**

### 1. **Hybrid Search Implementations** (Most Practical for Your Use Case)

| Repo | Description | Language |
|------|-------------|----------|
| **[qdrant/workshop-ultimate-hybrid-search](https://github.com/qdrant/workshop-ultimate-hybrid-search)** | Step-by-step hybrid search with Query API, measures effectiveness of various setups | Python |
| **[qdrant/examples](https://github.com/qdrant/examples)** | Collection of Qdrant tutorials including hybrid search, filtering, re-ranking | Python |
| **[msvensson222/semantic-fashion-search](https://github.com/msvensson222/semantic-fashion-search)** | Fashion e-commerce hybrid search combining dense vectors + keyword matching | Python |
| **[jhaayush2004/Hybrid-Retrieval-Systems](https://github.com/jhaayush2004/Hybrid-Retrieval-Systems)** | BM25 + Vectorstore retrieval with EnsembleRetriever | Python |
| **[quickwit-oss/tantivy](https://github.com/quickwit-oss/tantivy)** | Full-text search engine (the BM25 side of hybrid) - 2x faster than Lucene | Rust 🦀 |
| **[infiniflow/infinity](https://github.com/infiniflow/infinity)** | Dense + Sparse + Full-text + Tensor reranker all-in-one | C++/Python |

### 2. **BGE-M3 and Multi-Vector Models** (Dense + Sparse + ColBERT in One)

| Repo | Description | Language |
|------|-------------|----------|
| **[FlagOpen/FlagEmbedding](https://github.com/FlagOpen/FlagEmbedding)** | 🔥 BGE-M3 official repo - dense, sparse, ColBERT in single model | Python |
| **[vespa-engine/pyvespa BGE-M3 notebook](https://github.com/vespa-engine/pyvespa/blob/master/docs/sphinx/source/examples/mother-of-all-embedding-models-cloud.ipynb)** | BGE-M3 with Vespa for all three retrieval modes | Python |
| **[hffei/bge-m3-api](https://github.com/hffei/bge-m3-api)** | API wrapper for BGE-M3 | Python |

### 3. **Hard Negative Mining & Contrastive Learning**

| Repo | Description | Language |
|------|-------------|----------|
| **[junxia97/ProGCL](https://github.com/junxia97/ProGCL)** | ICML 2022 - Rethinking Hard Negative Mining in Graph Contrastive Learning | PyTorch |
| **[mala-lab/AUGCL](https://github.com/mala-lab/AUGCL)** | Affinity Uncertainty-based Hard Negative Mining | PyTorch |
| **[UKPLab/sentence-transformers](https://github.com/UKPLab/sentence-transformers)** | Official sentence-transformers with TripletLoss, ContrastiveLoss, MNR loss | Python |
| **[josimarz/hard-negative-mining](https://github.com/josimarz/hard-negative-mining)** | Hard negative mining strategies for embedding models | Python |

### 4. **ColBERT & Late Interaction Re-rankers**

| Repo | Description | Language |
|------|-------------|----------|
| **[stanford-futuredata/ColBERT](https://github.com/stanford-futuredata/ColBERT)** | Original ColBERT implementation | Python |
| **[answerdotai/rerankers](https://github.com/answerdotai/rerankers)** | Unified interface for various re-rankers (ColBERT, cross-encoders, etc.) | Python |
| **[neuml/txtai](https://github.com/neuml/txtai)** | All-in-one framework with SPLADE, ColBERT, reranking pipelines | Python |
| **[fertlopus/mediumLLMs (notebook)](https://github.com/fertlopus/mediumLLMs/blob/main/notebooks/retrievals/3_late_interaction_reranker.ipynb)** | Late interaction re-ranker demo | Python |

### 5. **Fashion/E-commerce Retrieval** (Attribute-Sensitive Domain)

| Repo | Description | Language |
|------|-------------|----------|
| **[ihciah/deep-fashion-retrieval](https://github.com/ihciah/deep-fashion-retrieval)** | Separates deep features + **color features** explicitly! 🎨 | PyTorch |
| **[sunn-e/DeepFashion-retrieval-2019](https://github.com/sunn-e/DeepFashion-retrieval-2019)** | Updated fashion retrieval with color feature extraction | PyTorch |
| **[Pyligent/Fashion-Image-Text-Multimodal-retrieval](https://github.com/Pyligent/Fashion-Image-Text-Multimodal-retrieval)** | Joint image + text fashion search | Python |
| **[XiaoxiaoGuo/fashion-retrieval](https://github.com/XiaoxiaoGuo/fashion-retrieval)** | Dialog-based interactive image retrieval (handles attribute refinement!) | PyTorch |

### 6. **SPLADE (Learned Sparse Retrieval)**

| Repo | Description | Language |
|------|-------------|----------|
| **[naver/splade](https://github.com/naver/splade)** | Official SPLADE implementation - sparse lexical expansion | Python |
| **[prithivida/Splade_PP_en_v1](https://huggingface.co/prithivida/Splade_PP_en_v1)** | Pre-trained SPLADE++ model on HuggingFace | - |

### 7. **Qdrant Rust Client** (For Your Rust Preference)

| Repo | Description | Language |
|------|-------------|----------|
| **[qdrant/rust-client](https://github.com/qdrant/rust-client)** | Official Qdrant Rust client with filtering examples | Rust 🦀 |

---

## 🎯 **Key Repos for Your Specific Problem**

For the "BLACK sand beach" vs "WHITE sand beach" problem specifically, I'd prioritize:

### **Most Relevant:**

1. **[ihciah/deep-fashion-retrieval](https://github.com/ihciah/deep-fashion-retrieval)** 
   - This one is gold! It explicitly extracts **color features separately** from semantic features
   - Uses `all_color_feat.npy` alongside `all_feat.npy`
   - Combines deep features with color-specific embeddings

2. **[FlagOpen/FlagEmbedding (BGE-M3)](https://github.com/FlagOpen/FlagEmbedding)**
   - Dense + sparse + ColBERT in one model
   - The sparse component helps with exact term matching (BLACK vs WHITE)
   - See `research/BGE_M3/` for fine-tuning examples

3. **[qdrant/workshop-ultimate-hybrid-search](https://github.com/qdrant/workshop-ultimate-hybrid-search)**
   - Shows how to measure hybrid search effectiveness
   - Data-driven approach to tuning dense vs sparse weights

4. **[UKPLab/sentence-transformers](https://github.com/UKPLab/sentence-transformers)**
   - For fine-tuning with hard negatives
   - `TripletLoss` with (anchor="black sand beach", positive="dark volcanic beach", negative="white sand beach")

---

## 🛠️ **Quick Architecture Recommendation**

Based on the repos above, here's what I'd build for Massif:

```
┌─────────────────────────────────────────────────────────────┐
│                     Query: "BLACK sand beach"               │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  1. PRE-PROCESS: Extract critical terms                     │
│     • colors: ["black"]                                     │
│     • antonyms: ["white", "light"]                          │
│     • semantic_query: "sand beach"                          │
└─────────────────────────────────────────────────────────────┘
                              │
            ┌─────────────────┼─────────────────┐
            ▼                 ▼                 ▼
     ┌──────────┐      ┌──────────┐      ┌──────────┐
     │  Dense   │      │  Sparse  │      │ Metadata │
     │  Search  │      │  (BM25/  │      │  Filter  │
     │ (BGE-M3) │      │  SPLADE) │      │ colors   │
     └──────────┘      └──────────┘      └──────────┘
            │                 │                 │
            └─────────────────┼─────────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  2. FUSION: Reciprocal Rank Fusion (RRF)                    │
│     • Combine dense + sparse scores                         │
│     • Apply metadata filter boost/penalty                   │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  3. RE-RANK: ColBERT or Cross-Encoder                       │
│     • Antonym penalty: doc has "white" → score *= 0.1       │
│     • Attribute match bonus: doc has "black" → score += 0.15│
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
                    Final Ranked Results
```

Want me to dive deeper into any of these repos or help you set up a specific implementation? 🚀
