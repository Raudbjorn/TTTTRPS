//! Query Pipeline
//!
//! Orchestrates the full query preprocessing flow:
//! 1. Normalize input (trim, lowercase)
//! 2. Apply typo correction
//! 3. Apply synonym expansion
//! 4. Generate search queries for both BM25 and vector paths

use super::config::PreprocessConfig;
use super::error::PreprocessResult;
use super::synonyms::{build_default_ttrpg_synonyms, ExpandedQuery, SynonymMap};
use super::typo::{Correction, TypoCorrector};

/// Complete query preprocessing pipeline.
pub struct QueryPipeline {
    typo_corrector: TypoCorrector,
    synonym_map: SynonymMap,
    config: PreprocessConfig,
}

/// Result of preprocessing a raw query.
#[derive(Debug, Clone)]
pub struct ProcessedQuery {
    /// Original user input
    pub original: String,
    /// Typo-corrected version
    pub corrected: String,
    /// Individual corrections made (for UI feedback)
    pub corrections: Vec<Correction>,
    /// Synonym-expanded query for BM25
    pub expanded: ExpandedQuery,
    /// Text to embed for vector search (corrected, not expanded)
    pub text_for_embedding: String,
}

impl QueryPipeline {
    /// Create pipeline with the given configuration.
    pub fn new(config: PreprocessConfig) -> PreprocessResult<Self> {
        let typo_corrector = TypoCorrector::new(config.typo.clone())?;

        // Load or build synonym map
        let mut synonym_map = if let Some(ref path) = config.synonyms.synonyms_path {
            if path.exists() {
                SynonymMap::from_toml_file(path)?
            } else {
                SynonymMap::new(config.synonyms.max_expansions)
            }
        } else {
            SynonymMap::new(config.synonyms.max_expansions)
        };

        // Merge default TTRPG synonyms if enabled
        if config.synonyms.use_default_ttrpg_synonyms {
            let defaults = build_default_ttrpg_synonyms();
            synonym_map.merge(&defaults);
        }

        Ok(Self {
            typo_corrector,
            synonym_map,
            config,
        })
    }

    /// Create pipeline with explicit components.
    pub fn from_components(
        typo_corrector: TypoCorrector,
        synonym_map: SynonymMap,
    ) -> Self {
        Self {
            typo_corrector,
            synonym_map,
            config: PreprocessConfig::default(),
        }
    }

    /// Create a minimal pipeline for testing.
    pub fn new_minimal() -> Self {
        Self {
            typo_corrector: TypoCorrector::new_empty(),
            synonym_map: build_default_ttrpg_synonyms(),
            config: PreprocessConfig::default(),
        }
    }

    /// Process a raw user query through the full pipeline.
    ///
    /// Steps:
    /// 1. Normalize input (trim, lowercase)
    /// 2. Apply typo correction
    /// 3. Apply synonym expansion on corrected text
    /// 4. Generate outputs for BM25 and vector search
    pub fn process(&self, raw_query: &str) -> ProcessedQuery {
        // 1. Normalize
        let normalized = self.normalize(raw_query);

        // 2. Typo correction
        let (corrected, corrections) = self.typo_corrector.correct_query(&normalized);

        // 3. Synonym expansion (on corrected text)
        let expanded = if self.config.synonyms.enabled {
            self.synonym_map.expand_query(&corrected)
        } else {
            ExpandedQuery {
                original: corrected.clone(),
                term_groups: corrected
                    .split_whitespace()
                    .map(|t| vec![t.to_string()])
                    .collect(),
            }
        };

        // 4. Generate embedding text (corrected, not expanded, to avoid noise)
        let text_for_embedding = corrected.clone();

        ProcessedQuery {
            original: raw_query.to_string(),
            corrected,
            corrections,
            expanded,
            text_for_embedding,
        }
    }

    /// Normalize the query: trim whitespace, collapse multiple spaces, lowercase
    fn normalize(&self, query: &str) -> String {
        query
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    }

    /// Reload dictionaries (call after corpus regeneration)
    pub fn reload_typo_dictionaries(&mut self) -> PreprocessResult<()> {
        self.typo_corrector.reload_dictionaries()
    }

    /// Add a synonym group at runtime
    pub fn add_synonyms_multi_way(&mut self, terms: &[&str]) {
        self.synonym_map.add_multi_way(terms);
    }

    /// Add a one-way synonym at runtime
    pub fn add_synonyms_one_way(&mut self, source: &str, targets: &[&str]) {
        self.synonym_map.add_one_way(source, targets);
    }

    /// Add a protected word that should never be corrected
    pub fn add_protected_word(&mut self, word: &str) {
        self.typo_corrector.add_protected_word(word);
    }

    /// Get a reference to the synonym map
    pub fn synonym_map(&self) -> &SynonymMap {
        &self.synonym_map
    }

    /// Get a reference to the typo corrector
    pub fn typo_corrector(&self) -> &TypoCorrector {
        &self.typo_corrector
    }
}

impl ProcessedQuery {
    /// Generate SurrealDB FTS query string
    pub fn to_surrealdb_fts(&self, field: &str, analyzer_ref: u32) -> String {
        self.expanded.to_surrealdb_fts(field, analyzer_ref)
    }

    /// Generate SQLite FTS5 MATCH expression
    pub fn to_fts5_match(&self) -> String {
        self.expanded.to_fts5_match()
    }

    /// Check if any corrections were made
    pub fn has_corrections(&self) -> bool {
        !self.corrections.is_empty()
    }

    /// Get a summary of corrections for display
    pub fn corrections_summary(&self) -> Option<String> {
        if self.corrections.is_empty() {
            return None;
        }

        let corrections: Vec<String> = self
            .corrections
            .iter()
            .map(|c| format!("{} → {}", c.original, c.corrected))
            .collect();

        Some(corrections.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize() {
        let pipeline = QueryPipeline::new_minimal();
        let result = pipeline.process("  Fireball   DAMAGE  ");

        assert_eq!(result.original, "  Fireball   DAMAGE  ");
        // After normalization (but no typo correction without dictionary)
        assert!(result.corrected.contains("fireball"));
        assert!(result.corrected.contains("damage"));
    }

    #[test]
    fn test_full_pipeline() {
        let pipeline = QueryPipeline::new_minimal();

        // "hp" should expand to include "hit points"
        let result = pipeline.process("restore hp");

        // Check expansion happened
        assert!(result.expanded.term_groups.len() >= 2);

        // The hp group should contain synonyms
        let hp_group = result.expanded.term_groups
            .iter()
            .find(|g| g.contains(&"hp".to_string()));

        assert!(hp_group.is_some());
        let hp_group = hp_group.unwrap();
        assert!(hp_group.contains(&"hit points".to_string()));
    }

    #[test]
    fn test_surrealdb_fts_generation() {
        let pipeline = QueryPipeline::new_minimal();
        let result = pipeline.process("restore hp");

        let fts = result.to_surrealdb_fts("content", 1);

        // Should have AND between term groups
        assert!(fts.contains(" AND "));
        // Should have OR for synonyms
        assert!(fts.contains(" OR ") || fts.contains("@1@"));
    }

    #[test]
    fn test_text_for_embedding() {
        let pipeline = QueryPipeline::new_minimal();
        let result = pipeline.process("restore hp");

        // Embedding text should be corrected but NOT expanded
        // (to avoid embedding multiple synonyms which adds noise)
        assert_eq!(result.text_for_embedding, "restore hp");
    }

    #[test]
    fn test_corrections_summary() {
        let result = ProcessedQuery {
            original: "firball damge".to_string(),
            corrected: "fireball damage".to_string(),
            corrections: vec![
                Correction {
                    original: "firball".to_string(),
                    corrected: "fireball".to_string(),
                    edit_distance: 1,
                },
                Correction {
                    original: "damge".to_string(),
                    corrected: "damage".to_string(),
                    edit_distance: 1,
                },
            ],
            expanded: ExpandedQuery {
                original: "fireball damage".to_string(),
                term_groups: vec![
                    vec!["fireball".to_string()],
                    vec!["damage".to_string()],
                ],
            },
            text_for_embedding: "fireball damage".to_string(),
        };

        assert!(result.has_corrections());
        let summary = result.corrections_summary().unwrap();
        assert!(summary.contains("firball → fireball"));
        assert!(summary.contains("damge → damage"));
    }

    #[test]
    fn test_add_runtime_synonyms() {
        let mut pipeline = QueryPipeline::new_minimal();

        // Add custom campaign-specific synonyms
        pipeline.add_synonyms_multi_way(&["bob", "bob the wizard", "archmage bob"]);

        let result = pipeline.process("where is bob");

        // Find the bob group
        let bob_group = result.expanded.term_groups
            .iter()
            .find(|g| g.contains(&"bob".to_string()));

        assert!(bob_group.is_some());
        let bob_group = bob_group.unwrap();
        assert!(bob_group.contains(&"archmage bob".to_string()));
    }
}
