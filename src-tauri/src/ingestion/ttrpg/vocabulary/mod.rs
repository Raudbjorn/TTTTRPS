//! Comprehensive TTRPG Vocabulary Module
//!
//! Defines game-specific vocabularies for different TTRPG systems.
//! Used by the attribute extractor to identify game-specific terms,
//! detect content categories, genres, and publishers for embedding enrichment.
//!
//! ## Module Structure
//!
//! - `data`: Static vocabulary lists (genres, classes, races, terms, etc.)
//! - `systems`: GameVocabulary trait and system implementations (D&D 5e, PF2e)
//! - `detection`: Genre, content category, and publisher detection
//! - `query`: Query expansion, spell correction, fuzzy matching, stop words
//! - `patterns`: Source book, header, and dice pattern detection
//! - `config`: Chunking and fusion configuration constants
//!
//! ## Usage
//!
//! ```rust,ignore
//! use crate::ingestion::ttrpg::vocabulary::{
//!     GameVocabulary, DnD5eVocabulary,
//!     detect_genre_from_vocabulary,
//!     expand_query,
//!     chunking_config,
//! };
//!
//! let vocab = DnD5eVocabulary;
//! let damage_types = vocab.damage_types();
//!
//! let genre = detect_genre_from_vocabulary("The wizard casts fireball...");
//! let expanded = expand_query("ac damage");
//! ```

pub mod config;
pub mod data;
pub mod detection;
pub mod patterns;
pub mod query;
pub mod systems;

// ============================================================================
// RE-EXPORTS - Data
// ============================================================================

pub use data::{
    // Genres
    GENRES,
    // Classes by genre
    FANTASY_CLASSES,
    HORROR_CLASSES,
    MODERN_CLASSES,
    SCIFI_CLASSES,
    // Races
    FANTASY_RACES,
    SCIFI_RACES,
    // System-specific terms
    BITD_TERMS,
    COC_TERMS,
    DELTA_GREEN_TERMS,
    DND5E_TERMS,
    FATE_TERMS,
    GURPS_TERMS,
    MOTHERSHIP_TERMS,
    PBTA_TERMS,
    PF2E_TERMS,
    SAVAGE_WORLDS_TERMS,
    TRAVELLER_TERMS,
    // Content indicators
    ADVENTURE_INDICATORS,
    BESTIARY_INDICATORS,
    PLAYER_OPTIONS_INDICATORS,
    RULEBOOK_INDICATORS,
    SETTING_INDICATORS,
    // Equipment
    ARMOR,
    WEAPONS,
    // Character traits
    BACKGROUNDS,
    MOTIVATIONS,
    // Publishers
    PUBLISHERS,
};

// ============================================================================
// RE-EXPORTS - Systems
// ============================================================================

pub use systems::{DnD5eVocabulary, GameVocabulary, Pf2eVocabulary};

// ============================================================================
// RE-EXPORTS - Detection
// ============================================================================

pub use detection::{
    count_vocabulary_matches, detect_content_category_from_vocabulary,
    detect_genre_from_vocabulary, detect_publisher_from_vocabulary, find_vocabulary_matches,
};

// ============================================================================
// RE-EXPORTS - Query Processing
// ============================================================================

pub use query::{
    // Vocabulary data
    BM25_STOP_WORDS,
    MECHANIC_TYPE_KEYWORDS,
    QUERY_EXPANSIONS,
    QUERY_SYNONYMS,
    TTRPG_CORE_VOCABULARY,
    // Functions
    correct_query_spelling,
    correct_spelling,
    detect_mechanic_type,
    expand_query,
    expand_query_term,
    extract_semantic_keywords,
    filter_stop_words,
    fuzzy_match,
    is_stop_word,
    levenshtein_distance,
};

// ============================================================================
// RE-EXPORTS - Patterns
// ============================================================================

pub use patterns::{
    // Pattern data
    DICE_PATTERNS,
    HEADER_PATTERNS,
    SOURCE_BOOK_PATTERNS,
    TABLE_ROW_PATTERNS,
    // Functions
    contains_dice_notation,
    count_dice_notation,
    detect_header_level,
    detect_source_book,
};

// ============================================================================
// RE-EXPORTS - Config
// ============================================================================

pub use config::{chunking_config, fusion_config};
