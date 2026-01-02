//! Query Enhancement Module
//!
//! Provides a unified interface for query enhancement including:
//! - Spell correction with TTRPG awareness
//! - Query expansion with TTRPG synonyms
//! - Autocomplete suggestions
//! - Clarification prompts for ambiguous queries
//! - Search hints for the UI

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Instant;

use super::synonyms::{TTRPGSynonyms, ClarificationPrompt, QueryExpansionResult, ExpansionInfo};
use crate::core::spell_correction::{SpellCorrector, CorrectionResult, SpellingSuggestion};

// ============================================================================
// Types
// ============================================================================

/// Complete result of query enhancement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedQuery {
    /// Original user query
    pub original: String,
    /// Final enhanced query for search
    pub enhanced: String,
    /// Whether any enhancement was applied
    pub was_enhanced: bool,
    /// Spell correction details
    pub correction: Option<CorrectionDetails>,
    /// Expansion details
    pub expansion: Option<ExpansionDetails>,
    /// Hints for the UI
    pub hints: Vec<SearchHint>,
    /// Clarification prompt (if query is ambiguous)
    pub clarification: Option<ClarificationPrompt>,
    /// Autocomplete suggestions based on partial input
    pub suggestions: Vec<String>,
    /// Processing time in microseconds
    pub processing_time_us: u64,
}

/// Details about spell correction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionDetails {
    /// Corrected query string
    pub corrected_query: String,
    /// Individual word corrections
    pub corrections: Vec<WordCorrection>,
    /// "Did you mean..." prompt
    pub did_you_mean: Option<String>,
}

/// A single word correction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordCorrection {
    /// Original word
    pub original: String,
    /// Corrected word
    pub corrected: String,
    /// Confidence (0.0 - 1.0)
    pub confidence: f64,
    /// Edit distance
    pub distance: usize,
}

/// Details about query expansion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpansionDetails {
    /// Expanded query string
    pub expanded_query: String,
    /// Terms that were expanded
    pub expansions: Vec<TermExpansion>,
    /// Whether abbreviations were expanded
    pub expanded_abbreviations: bool,
    /// Whether synonyms were added
    pub added_synonyms: bool,
}

/// A single term expansion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermExpansion {
    /// Original term
    pub original: String,
    /// Expanded to these terms
    pub expanded_to: Vec<String>,
    /// Type of expansion (abbreviation, synonym, related)
    pub expansion_type: String,
}

/// Search hint for the UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHint {
    /// Hint text
    pub text: String,
    /// Hint type (abbreviation, suggestion, tip)
    pub hint_type: HintType,
    /// Optional icon/badge
    pub icon: Option<String>,
}

/// Type of search hint
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HintType {
    /// Abbreviation explanation (HP = Hit Points)
    Abbreviation,
    /// Search suggestion
    Suggestion,
    /// General tip
    Tip,
    /// Correction hint
    Correction,
    /// Context help
    Context,
}

// ============================================================================
// Query Enhancer
// ============================================================================

/// Unified query enhancer combining spell correction, expansion, and suggestions
pub struct QueryEnhancer {
    /// TTRPG synonyms dictionary
    synonyms: TTRPGSynonyms,
    /// Spell corrector
    corrector: SpellCorrector,
    /// Enable spell correction
    spell_correction_enabled: bool,
    /// Enable query expansion
    expansion_enabled: bool,
    /// Enable clarification prompts
    clarification_enabled: bool,
    /// Minimum confidence for auto-correction
    min_correction_confidence: f64,
}

impl QueryEnhancer {
    /// Create a new query enhancer with default settings
    pub fn new() -> Self {
        Self {
            synonyms: TTRPGSynonyms::new(),
            corrector: SpellCorrector::new(),
            spell_correction_enabled: true,
            expansion_enabled: true,
            clarification_enabled: true,
            min_correction_confidence: 0.7,
        }
    }

    /// Create with custom configuration
    pub fn with_config(
        spell_correction: bool,
        expansion: bool,
        clarification: bool,
        min_confidence: f64,
    ) -> Self {
        Self {
            synonyms: TTRPGSynonyms::new(),
            corrector: SpellCorrector::new(),
            spell_correction_enabled: spell_correction,
            expansion_enabled: expansion,
            clarification_enabled: clarification,
            min_correction_confidence: min_confidence,
        }
    }

    /// Enhance a query with spell correction, expansion, and suggestions
    pub fn enhance(&self, query: &str) -> EnhancedQuery {
        let start = Instant::now();
        let mut hints = Vec::new();
        let mut current_query = query.to_string();
        let mut correction_details = None;
        let mut expansion_details = None;
        let mut was_enhanced = false;

        // Step 1: Check for TTRPG-specific typos first (higher priority)
        let mut ttrpg_corrections: Vec<WordCorrection> = Vec::new();
        for word in query.split_whitespace() {
            if let Some(correction) = self.synonyms.get_typo_correction(word) {
                ttrpg_corrections.push(WordCorrection {
                    original: word.to_string(),
                    corrected: correction.clone(),
                    confidence: 0.95, // High confidence for known TTRPG typos
                    distance: 1,
                });
            }
        }

        // Apply TTRPG typo corrections
        if !ttrpg_corrections.is_empty() {
            let mut corrected = current_query.clone();
            for correction in &ttrpg_corrections {
                corrected = corrected.replace(&correction.original, &correction.corrected);
            }
            current_query = corrected.clone();
            was_enhanced = true;

            hints.push(SearchHint {
                text: format!(
                    "Corrected: {}",
                    ttrpg_corrections
                        .iter()
                        .map(|c| format!("{} -> {}", c.original, c.corrected))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                hint_type: HintType::Correction,
                icon: Some("spell-check".to_string()),
            });
        }

        // Step 2: Apply general spell correction
        if self.spell_correction_enabled {
            let correction_result = self.corrector.correct(&current_query);
            if correction_result.has_corrections {
                let high_confidence: Vec<_> = correction_result
                    .corrections
                    .iter()
                    .filter(|c| c.confidence >= self.min_correction_confidence)
                    .collect();

                if !high_confidence.is_empty() {
                    current_query = correction_result.corrected_query.clone();
                    was_enhanced = true;

                    // Combine with TTRPG corrections
                    let all_corrections: Vec<WordCorrection> = ttrpg_corrections
                        .into_iter()
                        .chain(
                            correction_result.corrections.iter().map(|c| WordCorrection {
                                original: c.original.clone(),
                                corrected: c.suggestion.clone(),
                                confidence: c.confidence,
                                distance: c.distance,
                            }),
                        )
                        .collect();

                    correction_details = Some(CorrectionDetails {
                        corrected_query: correction_result.corrected_query.clone(),
                        corrections: all_corrections,
                        did_you_mean: if correction_result.has_corrections {
                            Some(format!("Did you mean: \"{}\"?", correction_result.corrected_query))
                        } else {
                            None
                        },
                    });
                }
            }
        }

        // Step 3: Apply query expansion
        if self.expansion_enabled {
            let expansion_result = self.synonyms.expand_query(&current_query);
            if expansion_result.was_expanded {
                current_query = expansion_result.expanded_query.clone();
                was_enhanced = true;

                // Add hints for expansions
                for hint in &expansion_result.hints {
                    hints.push(SearchHint {
                        text: hint.clone(),
                        hint_type: HintType::Abbreviation,
                        icon: Some("info".to_string()),
                    });
                }

                let term_expansions: Vec<TermExpansion> = expansion_result
                    .expansions
                    .iter()
                    .map(|e| TermExpansion {
                        original: e.original.clone(),
                        expanded_to: e.expanded_to.clone(),
                        expansion_type: e.category.clone(),
                    })
                    .collect();

                expansion_details = Some(ExpansionDetails {
                    expanded_query: expansion_result.expanded_query,
                    expansions: term_expansions.clone(),
                    expanded_abbreviations: term_expansions
                        .iter()
                        .any(|e| e.expansion_type == "abbreviation"),
                    added_synonyms: term_expansions.iter().any(|e| e.expansion_type == "synonym"),
                });
            }
        }

        // Step 4: Check for clarification needs
        let clarification = if self.clarification_enabled {
            self.synonyms.get_clarification(query)
        } else {
            None
        };

        if clarification.is_some() {
            hints.push(SearchHint {
                text: "This query may be ambiguous - see clarification options".to_string(),
                hint_type: HintType::Context,
                icon: Some("question".to_string()),
            });
        }

        // Step 5: Generate suggestions based on partial input
        let last_word = query.split_whitespace().last().unwrap_or("");
        let suggestions = self.get_suggestions(last_word, Some(query));

        // Add contextual tips
        self.add_contextual_hints(&current_query, &mut hints);

        let processing_time = start.elapsed().as_micros() as u64;

        EnhancedQuery {
            original: query.to_string(),
            enhanced: current_query,
            was_enhanced,
            correction: correction_details,
            expansion: expansion_details,
            hints,
            clarification,
            suggestions,
            processing_time_us: processing_time,
        }
    }

    /// Get autocomplete suggestions for partial input
    pub fn get_suggestions(&self, partial: &str, context: Option<&str>) -> Vec<String> {
        if partial.len() < 2 {
            return Vec::new();
        }

        let mut suggestions = self.synonyms.get_completions(partial, context);

        // Add "did you mean" suggestions if partial looks like a typo
        let did_you_mean = self.corrector.did_you_mean(partial);
        for suggestion in did_you_mean.into_iter().take(3) {
            if !suggestions.contains(&suggestion) {
                suggestions.push(suggestion);
            }
        }

        suggestions.truncate(10);
        suggestions
    }

    /// Get search hints for a query (without modifying it)
    pub fn get_hints(&self, query: &str) -> Vec<SearchHint> {
        let mut hints = Vec::new();

        // Check for TTRPG terms and add explanatory hints
        for word in query.split_whitespace() {
            if let Some(expansion) = self.synonyms.expand_abbreviation(word) {
                hints.push(SearchHint {
                    text: format!("{} = {}", word.to_uppercase(), expansion),
                    hint_type: HintType::Abbreviation,
                    icon: Some("info".to_string()),
                });
            }
        }

        // Add tips based on query content
        self.add_contextual_hints(query, &mut hints);

        hints
    }

    /// Add contextual hints based on query content
    fn add_contextual_hints(&self, query: &str, hints: &mut Vec<SearchHint>) {
        let query_lower = query.to_lowercase();

        // Combat-related tips
        if query_lower.contains("attack")
            || query_lower.contains("damage")
            || query_lower.contains("hit")
        {
            if !hints.iter().any(|h| h.text.contains("combat")) {
                hints.push(SearchHint {
                    text: "Tip: Use 'crit' for critical hits, 'aoe' for area of effect".to_string(),
                    hint_type: HintType::Tip,
                    icon: Some("sword".to_string()),
                });
            }
        }

        // Spell-related tips
        if query_lower.contains("spell")
            || query_lower.contains("magic")
            || query_lower.contains("cast")
        {
            if !hints.iter().any(|h| h.text.contains("spell")) {
                hints.push(SearchHint {
                    text: "Tip: Use 'conc' for concentration, 'rit' for ritual spells".to_string(),
                    hint_type: HintType::Tip,
                    icon: Some("wand".to_string()),
                });
            }
        }

        // Character-related tips
        if query_lower.contains("character")
            || query_lower.contains("class")
            || query_lower.contains("race")
        {
            if !hints.iter().any(|h| h.text.contains("character")) {
                hints.push(SearchHint {
                    text: "Tip: Ability scores: STR, DEX, CON, INT, WIS, CHA".to_string(),
                    hint_type: HintType::Tip,
                    icon: Some("user".to_string()),
                });
            }
        }

        // Monster-related tips
        if query_lower.contains("monster")
            || query_lower.contains("creature")
            || query_lower.contains("enemy")
        {
            if !hints.iter().any(|h| h.text.contains("monster")) {
                hints.push(SearchHint {
                    text: "Tip: Use 'CR' for challenge rating, filter by monster type".to_string(),
                    hint_type: HintType::Tip,
                    icon: Some("skull".to_string()),
                });
            }
        }
    }

    /// Expand a query with TTRPG synonyms (standalone)
    pub fn expand_query(&self, query: &str) -> QueryExpansionResult {
        self.synonyms.expand_query(query)
    }

    /// Correct a query (standalone)
    pub fn correct_query(&self, query: &str) -> CorrectionResult {
        self.corrector.correct(query)
    }

    /// Check if a term is a known TTRPG term
    pub fn is_ttrpg_term(&self, term: &str) -> bool {
        self.synonyms.is_ttrpg_term(term)
    }

    /// Get clarification for ambiguous query
    pub fn get_clarification(&self, query: &str) -> Option<ClarificationPrompt> {
        self.synonyms.get_clarification(query)
    }

    /// Enable or disable spell correction
    pub fn set_spell_correction(&mut self, enabled: bool) {
        self.spell_correction_enabled = enabled;
    }

    /// Enable or disable query expansion
    pub fn set_expansion(&mut self, enabled: bool) {
        self.expansion_enabled = enabled;
    }

    /// Enable or disable clarification prompts
    pub fn set_clarification(&mut self, enabled: bool) {
        self.clarification_enabled = enabled;
    }

    /// Set minimum confidence for auto-correction
    pub fn set_min_confidence(&mut self, confidence: f64) {
        self.min_correction_confidence = confidence.clamp(0.0, 1.0);
    }
}

impl Default for QueryEnhancer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Quick function to enhance a query with defaults
pub fn enhance_query(query: &str) -> EnhancedQuery {
    let enhancer = QueryEnhancer::new();
    enhancer.enhance(query)
}

/// Quick function to get suggestions
pub fn get_query_suggestions(partial: &str) -> Vec<String> {
    let enhancer = QueryEnhancer::new();
    enhancer.get_suggestions(partial, None)
}

/// Quick function to get hints
pub fn get_query_hints(query: &str) -> Vec<SearchHint> {
    let enhancer = QueryEnhancer::new();
    enhancer.get_hints(query)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhance_with_abbreviation() {
        let enhancer = QueryEnhancer::new();
        let result = enhancer.enhance("how much hp does a goblin have");

        assert!(result.was_enhanced);
        assert!(result.enhanced.contains("hit points"));
        assert!(!result.hints.is_empty());
    }

    #[test]
    fn test_enhance_with_typo() {
        let enhancer = QueryEnhancer::new();
        let result = enhancer.enhance("rouge attack");

        // Should correct "rouge" to "rogue"
        assert!(result.was_enhanced);
        assert!(result.enhanced.contains("rogue") || result.correction.is_some());
    }

    #[test]
    fn test_suggestions() {
        let enhancer = QueryEnhancer::new();
        let suggestions = enhancer.get_suggestions("hp", None);

        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.contains("hit points")));
    }

    #[test]
    fn test_hints() {
        let enhancer = QueryEnhancer::new();
        let hints = enhancer.get_hints("what is hp in combat");

        assert!(!hints.is_empty());
        assert!(hints.iter().any(|h| h.hint_type == HintType::Abbreviation));
    }

    #[test]
    fn test_clarification() {
        let enhancer = QueryEnhancer::new();
        let clarification = enhancer.get_clarification("what is int");

        assert!(clarification.is_some());
        let clarification = clarification.unwrap();
        assert!(!clarification.options.is_empty());
    }

    #[test]
    fn test_combined_enhancement() {
        let enhancer = QueryEnhancer::new();

        // Query with both typo and abbreviation
        let result = enhancer.enhance("fiorball dmg");

        assert!(result.was_enhanced);
        // Should expand dmg and potentially correct fiorball to fireball
    }

    #[test]
    fn test_contextual_hints() {
        let enhancer = QueryEnhancer::new();

        // Combat query should get combat tips
        let hints = enhancer.get_hints("attack roll damage");
        assert!(hints.iter().any(|h| h.hint_type == HintType::Tip));

        // Spell query should get spell tips
        let hints = enhancer.get_hints("fireball spell casting");
        assert!(hints.iter().any(|h| h.hint_type == HintType::Tip));
    }

    #[test]
    fn test_quick_functions() {
        let result = enhance_query("hp ac dc");
        assert!(result.was_enhanced);

        let suggestions = get_query_suggestions("hp");
        assert!(!suggestions.is_empty());

        let hints = get_query_hints("what is hp");
        assert!(!hints.is_empty());
    }

    #[test]
    fn test_config() {
        let mut enhancer = QueryEnhancer::with_config(false, true, true, 0.8);

        // Disable spell correction should not correct typos
        let result = enhancer.enhance("rouge");
        assert!(!result.correction.is_some() || !result.correction.as_ref().unwrap().corrections.iter().any(|c| c.corrected == "rogue"));

        // Enable and test again
        enhancer.set_spell_correction(true);
    }

    #[test]
    fn test_processing_time() {
        let enhancer = QueryEnhancer::new();
        let result = enhancer.enhance("test query");

        // Should have recorded processing time
        assert!(result.processing_time_us > 0);
    }
}
