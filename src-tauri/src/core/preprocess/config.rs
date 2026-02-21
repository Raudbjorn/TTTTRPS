//! Preprocessing Configuration
//!
//! Configuration structures for typo correction and synonym expansion,
//! with Meilisearch-compatible defaults.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Overall preprocessing configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreprocessConfig {
    /// Typo correction configuration
    #[serde(default)]
    pub typo: TypoConfig,

    /// Synonym expansion configuration
    #[serde(default)]
    pub synonyms: SynonymConfig,

    /// Base directory for data files (dictionaries, synonyms)
    #[serde(default)]
    pub data_dir: Option<PathBuf>,
}

impl Default for PreprocessConfig {
    fn default() -> Self {
        Self {
            typo: TypoConfig::default(),
            synonyms: SynonymConfig::default(),
            data_dir: None,
        }
    }
}

impl PreprocessConfig {
    /// Load configuration from TOML file
    pub fn from_toml_file(path: &std::path::Path) -> Result<Self, crate::core::preprocess::error::PreprocessError> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load from TOML string
    pub fn from_toml_str(content: &str) -> Result<Self, crate::core::preprocess::error::PreprocessError> {
        let config: Self = toml::from_str(content)?;
        Ok(config)
    }
}

/// Typo correction configuration matching Meilisearch's typo tolerance behavior
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TypoConfig {
    /// Whether typo correction is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Minimum word length to allow 1 typo (default: 5, Meilisearch default)
    #[serde(default = "default_min_one_typo")]
    pub min_word_size_one_typo: usize,

    /// Minimum word length to allow 2 typos (default: 9, Meilisearch default)
    #[serde(default = "default_min_two_typos")]
    pub min_word_size_two_typos: usize,

    /// Words where typo tolerance is disabled entirely (proper nouns, game terms)
    #[serde(default)]
    pub disabled_on_words: Vec<String>,

    /// Additional protected words that should never be corrected
    #[serde(default)]
    pub protected_words: Vec<String>,

    /// Path to English frequency dictionary
    #[serde(default)]
    pub english_dict_path: Option<PathBuf>,

    /// Path to TTRPG corpus dictionary (generated from indexed content)
    #[serde(default)]
    pub corpus_dict_path: Option<PathBuf>,

    /// Path to bigram dictionary for compound word correction
    #[serde(default)]
    pub bigram_dict_path: Option<PathBuf>,

    /// Frequency boost multiplier for corpus terms (default: 10)
    #[serde(default = "default_corpus_boost")]
    pub corpus_frequency_boost: u64,
}

impl Default for TypoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_word_size_one_typo: 5,  // Meilisearch default
            min_word_size_two_typos: 9, // Meilisearch default
            disabled_on_words: vec![
                // Common TTRPG abbreviations that shouldn't be corrected
                "dnd".to_string(),
                "5e".to_string(),
                "phb".to_string(),
                "dmg".to_string(),
                "mm".to_string(),
                "xge".to_string(),
                "tce".to_string(),
            ],
            protected_words: Vec::new(),
            english_dict_path: None,
            corpus_dict_path: None,
            bigram_dict_path: None,
            corpus_frequency_boost: 10,
        }
    }
}

impl TypoConfig {
    /// Creates a TypoConfig with dictionary paths auto-resolved.
    ///
    /// This resolves:
    /// - English dictionary from bundled resources or dev path
    /// - TTRPG corpus dictionary from user data directory
    /// - Bigram dictionary from user data directory
    ///
    /// Use this when initializing the application to get working paths.
    pub fn with_resolved_paths(app_handle: Option<&tauri::AppHandle>) -> Self {
        use super::paths::{
            get_bigram_dictionary_path, get_corpus_dictionary_path, get_english_dictionary_path,
        };

        Self {
            english_dict_path: get_english_dictionary_path(app_handle),
            corpus_dict_path: get_corpus_dictionary_path(),
            bigram_dict_path: get_bigram_dictionary_path(),
            ..Default::default()
        }
    }
}

/// Synonym expansion configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SynonymConfig {
    /// Whether synonym expansion is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Maximum number of synonym expansions per term (prevents query explosion)
    #[serde(default = "default_max_expansions")]
    pub max_expansions: usize,

    /// Path to synonyms TOML configuration file
    #[serde(default)]
    pub synonyms_path: Option<PathBuf>,

    /// Whether to use default TTRPG synonyms as fallback
    #[serde(default = "default_true")]
    pub use_default_ttrpg_synonyms: bool,
}

impl Default for SynonymConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_expansions: 5,
            synonyms_path: None,
            use_default_ttrpg_synonyms: true,
        }
    }
}

// Default value helpers for serde
fn default_true() -> bool { true }
fn default_min_one_typo() -> usize { 5 }
fn default_min_two_typos() -> usize { 9 }
fn default_max_expansions() -> usize { 5 }
fn default_corpus_boost() -> u64 { 10 }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PreprocessConfig::default();
        assert!(config.typo.enabled);
        assert_eq!(config.typo.min_word_size_one_typo, 5);
        assert_eq!(config.typo.min_word_size_two_typos, 9);
        assert!(config.synonyms.enabled);
        assert_eq!(config.synonyms.max_expansions, 5);
    }

    #[test]
    fn test_parse_toml() {
        let toml_str = r#"
[typo]
min_word_size_one_typo = 4
min_word_size_two_typos = 8
disabled_on_words = ["dnd", "5e", "phb"]

[synonyms]
max_expansions = 3
"#;
        let config = PreprocessConfig::from_toml_str(toml_str).unwrap();
        assert_eq!(config.typo.min_word_size_one_typo, 4);
        assert_eq!(config.typo.min_word_size_two_typos, 8);
        assert_eq!(config.typo.disabled_on_words.len(), 3);
        assert_eq!(config.synonyms.max_expansions, 3);
    }
}
