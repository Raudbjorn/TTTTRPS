//! Typo Correction using SymSpell
//!
//! Provides spelling correction for search queries using the SymSpell algorithm
//! with domain-specific TTRPG dictionaries layered on top of a base English dictionary.

use std::collections::HashSet;
use symspell::{SymSpell, UnicodeStringStrategy, Verbosity};

use super::config::TypoConfig;
use super::error::PreprocessResult;

/// A correction made to a word in the query
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Correction {
    /// Original word from the query
    pub original: String,
    /// Corrected word
    pub corrected: String,
    /// Edit distance between original and corrected
    pub edit_distance: usize,
}

/// Spelling correction engine with domain-specific vocabulary.
///
/// Uses SymSpell for fast, memory-efficient spell correction with layered dictionaries:
/// 1. English base dictionary (82K words)
/// 2. TTRPG corpus dictionary (generated from indexed content, 10x boost)
/// 3. Bigram dictionary for compound words
pub struct TypoCorrector {
    /// SymSpell engine for corrections
    engine: SymSpell<UnicodeStringStrategy>,
    /// Words that should never be corrected
    protected_words: HashSet<String>,
    /// Configuration
    config: TypoConfig,
}

impl TypoCorrector {
    /// Initialize with the given configuration.
    ///
    /// Loads dictionaries in order:
    /// 1. English frequency dictionary (base)
    /// 2. TTRPG corpus dictionary (if exists)
    /// 3. Bigram dictionary for compound words (if exists)
    pub fn new(config: TypoConfig) -> PreprocessResult<Self> {
        let mut engine: SymSpell<UnicodeStringStrategy> = SymSpell::default();

        // Load English base dictionary
        if let Some(ref english_path) = config.english_dict_path {
            if english_path.exists() {
                engine.load_dictionary(
                    english_path.to_string_lossy().as_ref(),
                    0,  // term_index
                    1,  // count_index
                    " " // separator
                );
            }
        }

        // Load TTRPG corpus dictionary with frequency boost
        if let Some(ref corpus_path) = config.corpus_dict_path {
            if corpus_path.exists() {
                // Load corpus terms (will be boosted in dictionary generation)
                engine.load_dictionary(
                    corpus_path.to_string_lossy().as_ref(),
                    0,
                    1,
                    " "
                );
            }
        }

        // Load bigram dictionary for compound words
        if let Some(ref bigram_path) = config.bigram_dict_path {
            if bigram_path.exists() {
                engine.load_bigram_dictionary(
                    bigram_path.to_string_lossy().as_ref(),
                    0,  // term_index
                    2,  // count_index
                    " " // separator
                );
            }
        }

        // Build protected words set
        let mut protected_words = HashSet::new();
        for word in &config.disabled_on_words {
            protected_words.insert(word.to_lowercase());
        }
        for word in &config.protected_words {
            protected_words.insert(word.to_lowercase());
        }

        Ok(Self {
            engine,
            protected_words,
            config,
        })
    }

    /// Create a new TypoCorrector with default settings and no dictionaries loaded.
    /// Useful for testing or when dictionaries aren't available.
    pub fn new_empty() -> Self {
        Self {
            engine: SymSpell::default(),
            protected_words: HashSet::new(),
            config: TypoConfig::default(),
        }
    }

    /// Reload dictionaries from disk.
    /// Call this after corpus dictionary regeneration.
    pub fn reload_dictionaries(&mut self) -> PreprocessResult<()> {
        // Create a new engine and reload all dictionaries
        let mut engine: SymSpell<UnicodeStringStrategy> = SymSpell::default();

        if let Some(ref english_path) = self.config.english_dict_path {
            if english_path.exists() {
                engine.load_dictionary(
                    english_path.to_string_lossy().as_ref(),
                    0, 1, " "
                );
            }
        }

        if let Some(ref corpus_path) = self.config.corpus_dict_path {
            if corpus_path.exists() {
                engine.load_dictionary(
                    corpus_path.to_string_lossy().as_ref(),
                    0, 1, " "
                );
            }
        }

        if let Some(ref bigram_path) = self.config.bigram_dict_path {
            if bigram_path.exists() {
                engine.load_bigram_dictionary(
                    bigram_path.to_string_lossy().as_ref(),
                    0, 2, " "
                );
            }
        }

        self.engine = engine;
        Ok(())
    }

    /// Correct a single word if it needs correction.
    ///
    /// Returns None if the word should not be changed (protected, too short, or already correct).
    fn correct_word(&self, word: &str) -> Option<Correction> {
        let word_lower = word.to_lowercase();
        let word_len = word_lower.chars().count();

        // Skip protected words
        if self.protected_words.contains(&word_lower) {
            return None;
        }

        // Determine max edit distance based on word length (Meilisearch-compatible rules)
        let max_edit_distance = if word_len < self.config.min_word_size_one_typo {
            return None; // Too short for any correction
        } else if word_len < self.config.min_word_size_two_typos {
            1 // Allow 1 typo
        } else {
            2 // Allow 2 typos
        };

        // Look up suggestions
        let suggestions = self.engine.lookup(&word_lower, Verbosity::Top, max_edit_distance as i64);

        if let Some(suggestion) = suggestions.first() {
            // Only return a correction if the suggestion is different
            if suggestion.term != word_lower && suggestion.distance > 0 {
                return Some(Correction {
                    original: word.to_string(),
                    corrected: suggestion.term.clone(),
                    edit_distance: suggestion.distance as usize,
                });
            }
        }

        None
    }

    /// Correct a full search query.
    ///
    /// Returns the corrected query string and a list of corrections made.
    pub fn correct_query(&self, query: &str) -> (String, Vec<Correction>) {
        if !self.config.enabled {
            return (query.to_string(), Vec::new());
        }

        let mut corrections = Vec::new();
        let mut corrected_words = Vec::new();

        // Split on whitespace while preserving word boundaries
        for word in query.split_whitespace() {
            if let Some(correction) = self.correct_word(word) {
                corrected_words.push(correction.corrected.clone());
                corrections.push(correction);
            } else {
                corrected_words.push(word.to_string());
            }
        }

        (corrected_words.join(" "), corrections)
    }

    /// Try to correct compound words (words that should be split).
    ///
    /// For example: "magicmissle" → "magic missile"
    pub fn correct_compound(&self, word: &str) -> Option<String> {
        let word_lower = word.to_lowercase();

        // Use SymSpell's word segmentation if bigrams are loaded
        let suggestions = self.engine.lookup_compound(&word_lower, 2);

        if let Some(suggestion) = suggestions.first() {
            if suggestion.term != word_lower && suggestion.term.contains(' ') {
                return Some(suggestion.term.clone());
            }
        }

        None
    }

    /// Add a protected word that should never be corrected
    pub fn add_protected_word(&mut self, word: &str) {
        self.protected_words.insert(word.to_lowercase());
    }

    /// Check if a word is protected from correction
    pub fn is_protected(&self, word: &str) -> bool {
        self.protected_words.contains(&word.to_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_words_not_corrected() {
        let corrector = TypoCorrector::new_empty();
        let (corrected, corrections) = corrector.correct_query("the cat sat");
        assert_eq!(corrected, "the cat sat");
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_protected_words_not_corrected() {
        let mut config = TypoConfig::default();
        config.disabled_on_words = vec!["phb".to_string(), "dmg".to_string()];
        let corrector = TypoCorrector::new(config).unwrap();

        // "phb" should not be corrected even though it might look like a typo
        assert!(corrector.is_protected("phb"));
        assert!(corrector.is_protected("PHB"));
    }

    #[test]
    fn test_add_protected_word() {
        let mut corrector = TypoCorrector::new_empty();
        corrector.add_protected_word("Tiamat");
        assert!(corrector.is_protected("tiamat"));
        assert!(corrector.is_protected("TIAMAT"));
    }

    #[test]
    fn test_correction_struct() {
        let correction = Correction {
            original: "firball".to_string(),
            corrected: "fireball".to_string(),
            edit_distance: 1,
        };
        assert_eq!(correction.original, "firball");
        assert_eq!(correction.corrected, "fireball");
        assert_eq!(correction.edit_distance, 1);
    }
}
