//! Query Preprocessing Module
//!
//! Provides typo correction and synonym expansion for search queries.
//! This eliminates the need for Meilisearch's built-in typo/synonym features
//! while giving us more control over domain-specific TTRPG behavior.
//!
//! ## Architecture
//!
//! ```text
//! User Query: "firball damge resistence"
//!        │
//!        ▼
//! ┌──────────────────────────────┐
//! │  1. Normalize                │  → "firball damge resistence"
//! │     (trim, lowercase)        │
//! └──────────────┬───────────────┘
//!                ▼
//! ┌──────────────────────────────┐
//! │  2. Typo Correction          │  "firball" → "fireball"
//! │     (SymSpell + corpus)      │  "damge" → "damage"
//! │                              │  "resistence" → "resistance"
//! └──────────────┬───────────────┘
//!                ▼
//! ┌──────────────────────────────┐
//! │  3. Synonym Expansion        │  "fireball" → ["fireball", "fire bolt"]
//! │     (domain dictionary)      │  "damage" → ["damage", "harm"]
//! └──────────────┬───────────────┘
//!                ▼
//! ┌──────────────────────────────────────────────────┐
//! │  4. Query Generation                              │
//! │  ┌─────────────────┐  ┌────────────────────────┐ │
//! │  │ BM25 FTS Query  │  │ Embedding Text         │ │
//! │  │ (OR-expanded)   │  │ (corrected only)       │ │
//! │  └─────────────────┘  └────────────────────────┘ │
//! └──────────────────────────────────────────────────┘
//! ```

pub mod config;
pub mod dictionary;
pub mod error;
pub mod paths;
pub mod pipeline;
pub mod rebuild;
pub mod synonyms;
pub mod typo;

// Re-export primary types
pub use config::{PreprocessConfig, SynonymConfig, TypoConfig};
pub use dictionary::DictionaryGenerator;
pub use error::PreprocessError;
pub use paths::{
    ensure_user_data_dir, get_bigram_dictionary_path, get_corpus_dictionary_path,
    get_english_dictionary_path, get_user_data_dir, BIGRAM_DICT_FILENAME, CORPUS_DICT_FILENAME,
    ENGLISH_DICT_FILENAME,
};
pub use pipeline::{ProcessedQuery, QueryPipeline};
pub use rebuild::{
    DictionaryRebuildService, RebuildConfig, RebuildStats,
    force_dictionary_rebuild, spawn_dictionary_rebuild,
};
pub use synonyms::{ExpandedQuery, SynonymMap};
pub use typo::{Correction, TypoCorrector};
