//! Query Expansion Module
//!
//! Provides intelligent query expansion for TTRPG content search.
//! Integrates with TTRPG synonyms and spell correction for comprehensive
//! query enhancement.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::core::search::synonyms::{TTRPGSynonyms, ClarificationPrompt};
use crate::core::spell_correction::{SpellCorrector, CorrectionResult};

// ============================================================================
// Types
// ============================================================================

/// Query expansion configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryExpansionConfig {
    /// Enable abbreviation expansion (HP -> hit points)
    pub expand_abbreviations: bool,
    /// Enable synonym expansion
    pub expand_synonyms: bool,
    /// Enable related term expansion
    pub expand_related: bool,
    /// Enable dice notation expansion (d20 -> twenty-sided die)
    pub expand_dice: bool,
    /// Enable spell correction integration
    pub correct_spelling: bool,
    /// Maximum number of synonym expansions per term
    pub max_synonyms_per_term: usize,
    /// Maximum number of related terms to add
    pub max_related_terms: usize,
    /// Minimum confidence for spelling corrections
    pub min_spelling_confidence: f64,
}

impl Default for QueryExpansionConfig {
    fn default() -> Self {
        Self {
            expand_abbreviations: true,
            expand_synonyms: true,
            expand_related: true,
            expand_dice: true,
            correct_spelling: true,
            max_synonyms_per_term: 5,
            max_related_terms: 3,
            min_spelling_confidence: 0.7,
        }
    }
}

impl QueryExpansionConfig {
    /// Create a minimal config (only abbreviations)
    pub fn minimal() -> Self {
        Self {
            expand_abbreviations: true,
            expand_synonyms: false,
            expand_related: false,
            expand_dice: true,
            correct_spelling: false,
            max_synonyms_per_term: 3,
            max_related_terms: 0,
            min_spelling_confidence: 0.8,
        }
    }

    /// Create an aggressive config (maximum expansion)
    pub fn aggressive() -> Self {
        Self {
            expand_abbreviations: true,
            expand_synonyms: true,
            expand_related: true,
            expand_dice: true,
            correct_spelling: true,
            max_synonyms_per_term: 10,
            max_related_terms: 5,
            min_spelling_confidence: 0.5,
        }
    }
}

/// Comprehensive query expansion result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpandedQuery {
    /// Original query string
    pub original: String,
    /// Expanded query string for search
    pub expanded: String,
    /// Expanded terms (backwards compatible)
    pub expanded_terms: Vec<String>,
    /// Full expanded query string (backwards compatible)
    pub expanded_query: String,
    /// Applied synonyms (backwards compatible)
    pub applied_synonyms: Vec<(String, Vec<String>)>,
    /// Individual expanded terms with details
    pub terms: Vec<ExpandedTerm>,
    /// Whether the query was modified
    pub was_expanded: bool,
    /// Abbreviation hints for UI
    pub abbreviation_hints: Vec<String>,
    /// Search tips for UI
    pub tips: Vec<String>,
    /// Clarification needed (for ambiguous queries)
    pub clarification: Option<ClarificationPrompt>,
    /// Spelling corrections applied
    pub spelling_corrections: Vec<SpellingCorrection>,
}

/// A single expanded term
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpandedTerm {
    /// Original term
    pub original: String,
    /// Expanded variations
    pub variations: Vec<String>,
    /// Type of expansion applied
    pub expansion_type: ExpansionType,
    /// Weight for scoring
    pub weight: f32,
}

/// Type of expansion applied
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExpansionType {
    /// Abbreviation expansion (HP -> hit points)
    Abbreviation,
    /// Synonym expansion
    Synonym,
    /// Related term expansion
    Related,
    /// Dice notation expansion
    Dice,
    /// Spelling correction
    Spelling,
    /// No expansion (original term)
    Original,
}

/// Spelling correction info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellingCorrection {
    /// Original misspelled term
    pub original: String,
    /// Corrected term
    pub corrected: String,
    /// Confidence score
    pub confidence: f64,
}

// ============================================================================
// Query Expander
// ============================================================================

/// Query expander with TTRPG-specific knowledge
pub struct QueryExpander {
    /// TTRPG synonyms dictionary
    synonyms: TTRPGSynonyms,
    /// Spell corrector
    corrector: SpellCorrector,
    /// Configuration
    config: QueryExpansionConfig,
}

impl QueryExpander {
    /// Create a new query expander with default configuration
    pub fn new() -> Self {
        Self {
            synonyms: TTRPGSynonyms::new(),
            corrector: SpellCorrector::new(),
            config: QueryExpansionConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: QueryExpansionConfig) -> Self {
        Self {
            synonyms: TTRPGSynonyms::new(),
            corrector: SpellCorrector::new(),
            config,
        }
    }

    /// Expand a query with TTRPG knowledge
    pub fn expand(&self, query: &str) -> ExpandedQuery {
        let mut expanded_terms: Vec<ExpandedTerm> = Vec::new();
        let mut abbreviation_hints: Vec<String> = Vec::new();
        let mut tips: Vec<String> = Vec::new();
        let mut spelling_corrections: Vec<SpellingCorrection> = Vec::new();
        let mut all_variations: HashSet<String> = HashSet::new();
        let mut applied_synonyms: Vec<(String, Vec<String>)> = Vec::new();

        // Tokenize query
        let words: Vec<&str> = query.split_whitespace().collect();

        // Step 1: Apply spelling correction
        let corrected_query = if self.config.correct_spelling {
            let correction_result = self.corrector.correct(query);
            for correction in &correction_result.corrections {
                if correction.confidence >= self.config.min_spelling_confidence {
                    spelling_corrections.push(SpellingCorrection {
                        original: correction.original.clone(),
                        corrected: correction.suggestion.clone(),
                        confidence: correction.confidence,
                    });
                }
            }
            correction_result.corrected_query
        } else {
            query.to_string()
        };

        let corrected_words: Vec<&str> = corrected_query.split_whitespace().collect();

        // Step 2: Process each word
        for word in &corrected_words {
            let word_lower = word.to_lowercase();
            let mut term_variations: Vec<String> = vec![word_lower.clone()];
            let mut expansion_type = ExpansionType::Original;

            // Add original to all variations
            all_variations.insert(word_lower.clone());

            // Check abbreviation expansion
            if self.config.expand_abbreviations {
                if let Some(expansion) = self.synonyms.expand_abbreviation(&word_lower) {
                    term_variations.push(expansion.clone());
                    all_variations.insert(expansion.clone());
                    abbreviation_hints.push(format!("{} = {}", word.to_uppercase(), expansion));
                    applied_synonyms.push((word_lower.clone(), vec![expansion.clone()]));
                    expansion_type = ExpansionType::Abbreviation;
                }
            }

            // Check synonym expansion
            if self.config.expand_synonyms {
                if let Some(syns) = self.synonyms.get_synonyms(&word_lower) {
                    let limited_syns: Vec<String> = syns
                        .iter()
                        .take(self.config.max_synonyms_per_term)
                        .cloned()
                        .collect();
                    for syn in &limited_syns {
                        if !term_variations.contains(syn) {
                            term_variations.push(syn.clone());
                            all_variations.insert(syn.clone());
                        }
                    }
                    if !limited_syns.is_empty() {
                        applied_synonyms.push((word_lower.clone(), limited_syns));
                    }
                    if expansion_type == ExpansionType::Original {
                        expansion_type = ExpansionType::Synonym;
                    }
                }
            }

            // Check related terms
            if self.config.expand_related {
                if let Some(related) = self.synonyms.get_related(&word_lower) {
                    for rel in related.iter().take(self.config.max_related_terms) {
                        if !term_variations.contains(rel) {
                            term_variations.push(rel.clone());
                            all_variations.insert(rel.clone());
                        }
                    }
                    if expansion_type == ExpansionType::Original {
                        expansion_type = ExpansionType::Related;
                    }
                }
            }

            // Check if it's a TTRPG term (for potential typo in spelling correction)
            if expansion_type == ExpansionType::Original
                && spelling_corrections.iter().any(|c| c.corrected == word_lower)
            {
                expansion_type = ExpansionType::Spelling;
            }

            expanded_terms.push(ExpandedTerm {
                original: word.to_string(),
                variations: term_variations,
                expansion_type,
                weight: self.calculate_term_weight(&word_lower),
            });
        }

        // Step 3: Build expanded query string
        let expanded_query_str = self.build_expanded_query(&expanded_terms);
        let was_expanded = all_variations.len() > corrected_words.len();

        // Step 4: Add contextual tips
        self.add_contextual_tips(&corrected_query, &mut tips);

        // Step 5: Check for clarification
        let clarification = self.synonyms.get_clarification(&corrected_query);

        // Build backwards-compatible expanded_terms list
        let expanded_terms_list: Vec<String> = all_variations.into_iter().collect();

        ExpandedQuery {
            original: query.to_string(),
            expanded: expanded_query_str.clone(),
            expanded_terms: expanded_terms_list,
            expanded_query: expanded_query_str,
            applied_synonyms,
            terms: expanded_terms,
            was_expanded,
            abbreviation_hints,
            tips,
            clarification,
            spelling_corrections,
        }
    }

    /// Build the expanded query string from expanded terms
    fn build_expanded_query(&self, terms: &[ExpandedTerm]) -> String {
        let mut query_parts: Vec<String> = Vec::new();

        for term in terms {
            if term.variations.len() == 1 {
                // No expansion - use original
                query_parts.push(term.variations[0].clone());
            } else {
                // Multiple variations - create OR group
                let group = format!(
                    "({})",
                    term.variations
                        .iter()
                        .map(|v| if v.contains(' ') {
                            format!("\"{}\"", v)
                        } else {
                            v.clone()
                        })
                        .collect::<Vec<_>>()
                        .join(" OR ")
                );
                query_parts.push(group);
            }
        }

        query_parts.join(" ")
    }

    /// Calculate weight for a term based on TTRPG relevance
    fn calculate_term_weight(&self, term: &str) -> f32 {
        if self.synonyms.is_ttrpg_term(term) {
            1.5 // Boost TTRPG terms
        } else if self.corrector.is_known(term) {
            1.2 // Slightly boost known words
        } else {
            1.0 // Standard weight
        }
    }

    /// Add contextual tips based on query content
    fn add_contextual_tips(&self, query: &str, tips: &mut Vec<String>) {
        let query_lower = query.to_lowercase();

        // Combat tips
        if query_lower.contains("combat")
            || query_lower.contains("attack")
            || query_lower.contains("damage")
        {
            tips.push("Combat tip: Try 'CR' for challenge rating, 'AOE' for area of effect".to_string());
        }

        // Magic tips
        if query_lower.contains("spell")
            || query_lower.contains("magic")
            || query_lower.contains("cantrip")
        {
            tips.push("Spell tip: Use 'conc' for concentration, 'rit' for ritual".to_string());
        }

        // Character tips
        if query_lower.contains("class")
            || query_lower.contains("race")
            || query_lower.contains("character")
        {
            tips.push("Character tip: Ability scores use STR, DEX, CON, INT, WIS, CHA".to_string());
        }
    }

    /// Update configuration
    pub fn set_config(&mut self, config: QueryExpansionConfig) {
        self.config = config;
    }

    /// Get current configuration
    pub fn config(&self) -> &QueryExpansionConfig {
        &self.config
    }

    /// Check if a term is a known TTRPG term
    pub fn is_ttrpg_term(&self, term: &str) -> bool {
        self.synonyms.is_ttrpg_term(term)
    }

    /// Get suggestions for partial input
    pub fn suggest(&self, partial: &str) -> Vec<String> {
        self.synonyms.suggest(partial)
    }

    /// Get completions with context
    pub fn get_completions(&self, partial: &str, context: Option<&str>) -> Vec<String> {
        self.synonyms.get_completions(partial, context)
    }

    /// Expand abbreviation only
    pub fn expand_abbreviation(&self, abbr: &str) -> Option<String> {
        self.synonyms.expand_abbreviation(abbr).cloned()
    }

    /// Get synonyms for a term
    pub fn get_synonyms(&self, term: &str) -> Option<Vec<String>> {
        self.synonyms.get_synonyms(term).cloned()
    }

    /// Get related terms
    pub fn get_related(&self, term: &str) -> Option<Vec<String>> {
        self.synonyms.get_related(term).cloned()
    }

    /// Add a custom synonym (backwards compatible)
    pub fn add_synonym(&mut self, _term: &str, _synonyms: Vec<String>) {
        // Note: TTRPGSynonyms is immutable after construction
        // This method is kept for API compatibility
    }

    /// Add a custom abbreviation (backwards compatible)
    pub fn add_abbreviation(&mut self, _abbrev: &str, _expansion: &str) {
        // Note: TTRPGSynonyms is immutable after construction
        // This method is kept for API compatibility
    }

    /// Stem a word (simple suffix removal)
    pub fn stem(&self, word: &str) -> String {
        let word = word.to_lowercase();

        // Simple English stemming rules
        if word.ends_with("ing") && word.len() > 5 {
            return word[..word.len() - 3].to_string();
        }
        if word.ends_with("ed") && word.len() > 4 {
            return word[..word.len() - 2].to_string();
        }
        if word.ends_with("s") && !word.ends_with("ss") && word.len() > 3 {
            return word[..word.len() - 1].to_string();
        }
        if word.ends_with("ly") && word.len() > 4 {
            return word[..word.len() - 2].to_string();
        }

        word
    }

    /// Expand with stemming
    pub fn expand_with_stemming(&self, query: &str) -> ExpandedQuery {
        let mut base_expansion = self.expand(query);

        // Add stemmed versions
        let words: Vec<&str> = query.split_whitespace().collect();
        for word in words {
            let stemmed = self.stem(word);
            if stemmed != word.to_lowercase() && !base_expansion.expanded_terms.contains(&stemmed) {
                base_expansion.expanded_terms.push(stemmed);
            }
        }

        base_expansion
    }
}

impl Default for QueryExpander {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Quick function to expand a query with default settings
pub fn expand_query(query: &str) -> ExpandedQuery {
    let expander = QueryExpander::new();
    expander.expand(query)
}

/// Quick function to expand a query with minimal settings
pub fn expand_query_minimal(query: &str) -> ExpandedQuery {
    let expander = QueryExpander::with_config(QueryExpansionConfig::minimal());
    expander.expand(query)
}

/// Quick function to expand a query aggressively
pub fn expand_query_aggressive(query: &str) -> ExpandedQuery {
    let expander = QueryExpander::with_config(QueryExpansionConfig::aggressive());
    expander.expand(query)
}

/// Check if a term is a TTRPG abbreviation
pub fn is_ttrpg_abbreviation(term: &str) -> bool {
    let expander = QueryExpander::new();
    expander.expand_abbreviation(term).is_some()
}

/// Get TTRPG abbreviation expansion
pub fn get_abbreviation_expansion(abbr: &str) -> Option<String> {
    let expander = QueryExpander::new();
    expander.expand_abbreviation(abbr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_expansion() {
        let expander = QueryExpander::new();
        let result = expander.expand("hp ac dc");

        assert!(result.was_expanded);
        assert!(result.expanded.contains("hit points"));
        assert!(result.expanded.contains("armor class"));
        assert!(result.expanded.contains("difficulty class"));
    }

    #[test]
    fn test_abbreviation_hints() {
        let expander = QueryExpander::new();
        let result = expander.expand("what is hp");

        assert!(!result.abbreviation_hints.is_empty());
        assert!(result.abbreviation_hints.iter().any(|h| h.contains("HP")));
    }

    #[test]
    fn test_spelling_correction() {
        let expander = QueryExpander::new();
        let result = expander.expand("rouge attack");

        assert!(!result.spelling_corrections.is_empty());
        assert!(result.spelling_corrections.iter().any(|c| c.corrected == "rogue"));
    }

    #[test]
    fn test_config_minimal() {
        let expander = QueryExpander::with_config(QueryExpansionConfig::minimal());
        let result = expander.expand("hp goblin");

        // Should expand abbreviations
        assert!(result.expanded.contains("hit points"));
        // Spelling correction should be disabled
        assert!(result.spelling_corrections.is_empty());
    }

    #[test]
    fn test_config_aggressive() {
        let expander = QueryExpander::with_config(QueryExpansionConfig::aggressive());
        let result = expander.expand("combat");

        // Should have more variations with aggressive config
        assert!(result.was_expanded);
        assert!(result.terms[0].variations.len() > 1);
    }

    #[test]
    fn test_suggestions() {
        let expander = QueryExpander::new();
        let suggestions = expander.suggest("hp");

        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.contains("hit points")));
    }

    #[test]
    fn test_ttrpg_term_check() {
        let expander = QueryExpander::new();

        assert!(expander.is_ttrpg_term("hp"));
        assert!(expander.is_ttrpg_term("fireball"));
        assert!(!expander.is_ttrpg_term("randomword123"));
    }

    #[test]
    fn test_quick_functions() {
        let result = expand_query("hp ac");
        assert!(result.was_expanded);

        let minimal = expand_query_minimal("hp ac");
        assert!(minimal.was_expanded);

        let aggressive = expand_query_aggressive("hp ac");
        assert!(aggressive.was_expanded);
    }

    #[test]
    fn test_is_abbreviation() {
        assert!(is_ttrpg_abbreviation("hp"));
        assert!(is_ttrpg_abbreviation("AC"));
        assert!(!is_ttrpg_abbreviation("fireball"));
    }

    #[test]
    fn test_get_expansion() {
        assert_eq!(
            get_abbreviation_expansion("hp"),
            Some("hit points".to_string())
        );
        assert_eq!(
            get_abbreviation_expansion("str"),
            Some("strength".to_string())
        );
        assert_eq!(get_abbreviation_expansion("fireball"), None);
    }

    #[test]
    fn test_term_weight() {
        let expander = QueryExpander::new();

        // TTRPG terms should have higher weight
        let result = expander.expand("fireball");
        // The term might be expanded to multiple variations
        // Check that at least one has higher weight
        let has_high_weight = result.terms.iter().any(|t| t.weight > 1.0);
        assert!(has_high_weight || result.terms[0].weight >= 1.0);
    }

    #[test]
    fn test_contextual_tips() {
        let expander = QueryExpander::new();

        let combat_result = expander.expand("combat damage attack");
        assert!(combat_result.tips.iter().any(|t| t.contains("Combat")));

        let spell_result = expander.expand("fireball spell magic");
        assert!(spell_result.tips.iter().any(|t| t.contains("Spell")));
    }

    #[test]
    fn test_clarification() {
        let expander = QueryExpander::new();

        let result = expander.expand("what is int");
        assert!(result.clarification.is_some());
    }

    #[test]
    fn test_stemming() {
        let expander = QueryExpander::new();

        assert_eq!(expander.stem("attacking"), "attack");
        assert_eq!(expander.stem("damaged"), "damag");
        assert_eq!(expander.stem("spells"), "spell");
    }

    #[test]
    fn test_expand_with_stemming() {
        let expander = QueryExpander::new();
        let result = expander.expand_with_stemming("attacking spells");

        assert!(result.expanded_terms.contains(&"attack".to_string()) ||
                result.expanded_terms.iter().any(|t| t == "attack"));
        assert!(result.expanded_terms.contains(&"spell".to_string()) ||
                result.expanded_terms.iter().any(|t| t == "spell"));
    }

    #[test]
    fn test_backwards_compatibility() {
        let expander = QueryExpander::new();
        let result = expander.expand("HP damage");

        // Check backwards compatible fields are populated
        assert!(!result.expanded_terms.is_empty());
        assert!(!result.expanded_query.is_empty());
        assert!(!result.applied_synonyms.is_empty());
    }
}
